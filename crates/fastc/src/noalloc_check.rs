//! `@noalloc` enforcement lint.
//!
//! For every function decorated with `@noalloc`, walk its
//! transitive call set and fail compilation if any reached function
//! is on the allocator banned-list (the prelude's `mem::alloc` /
//! `mem::resize` / `mem::free_bytes` plus the libc externs they
//! wrap).
//!
//! Algorithm:
//!
//! 1. Build a per-function "outgoing calls" map by walking every
//!    `Expr::Call { callee: Ident { name } }` in the body.
//! 2. For each fn with `@noalloc` in its annotations, compute the
//!    transitive closure of outgoing calls (BFS).
//! 3. If the closure intersects the banned-list, emit one
//!    diagnostic per banned reach, listing the entry point and the
//!    reached callee.
//!
//! Limitations of v1:
//!
//! - Indirect calls via fn pointers (`map(v, dyn_fn)`) aren't
//!   resolved — the analysis treats fn-pointer arguments as opaque.
//!   A future sub-slice can refine via a points-to analysis.
//! - Cross-mod calls are tracked by *qualified* name (e.g.
//!   `mem::alloc`) which mirrors how the post-deferred-1 parser
//!   produces idents.

use crate::ast::{Block, ElseBranch, Expr, File, FnDecl, ForInit, ForStep, Item, Stmt};
use crate::diag::CompileError;

/// Names that are considered allocation entry points. Both bare and
/// qualified forms are listed because callers may write either.
const BANNED: &[&str] = &[
    "alloc",
    "resize",
    "free_bytes",
    "mem::alloc",
    "mem::resize",
    "mem::free_bytes",
    // libc externs declared inside `mod mem`. These appear as
    // qualified `mem::malloc` etc. inside the prelude body.
    "malloc",
    "realloc",
    "free",
    "mem::malloc",
    "mem::realloc",
    "mem::free",
];

pub fn check_noalloc(file: &File, source: &str) -> Result<(), CompileError> {
    let mut all_fns: Vec<(Vec<String>, FnDecl)> = Vec::new();
    collect_fns(&file.items, &[], &mut all_fns);

    // Build an outgoing-calls map: fn_name -> Vec<callee_name>.
    let mut outgoing: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    for (_path, f) in &all_fns {
        let mut calls = Vec::new();
        walk_block_for_calls(&f.body, &mut calls);
        outgoing.insert(f.name.clone(), calls);
    }

    let mut errors: Vec<CompileError> = Vec::new();
    for (_path, f) in &all_fns {
        if !f.annotations.iter().any(|a| a == "noalloc") {
            continue;
        }
        let banned_reach = transitive_banned_reach(&f.name, &outgoing);
        for reached in banned_reach {
            errors.push(CompileError::resolve(
                format!(
                    "@noalloc function '{}' reaches '{}' (transitive call). Either drop the @noalloc annotation or refactor to avoid the heap allocator.",
                    f.name, reached
                ),
                f.span.clone(),
                source,
            ));
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(CompileError::multiple(errors))
    }
}

fn collect_fns(items: &[Item], path: &[String], out: &mut Vec<(Vec<String>, FnDecl)>) {
    for item in items {
        match item {
            Item::Fn(f) => out.push((path.to_vec(), f.clone())),
            Item::Mod(m) => {
                if let Some(body) = &m.body {
                    let mut p = path.to_vec();
                    p.push(m.name.clone());
                    collect_fns(body, &p, out);
                }
            }
            Item::Impl(block) => {
                for method in &block.methods {
                    out.push((path.to_vec(), method.clone()));
                }
            }
            _ => {}
        }
    }
}

fn walk_block_for_calls(b: &Block, out: &mut Vec<String>) {
    for s in &b.stmts {
        walk_stmt_for_calls(s, out);
    }
}

fn walk_stmt_for_calls(stmt: &Stmt, out: &mut Vec<String>) {
    match stmt {
        Stmt::Let { init, .. } => walk_expr_for_calls(init, out),
        Stmt::Assign { lhs, rhs, .. } => {
            walk_expr_for_calls(lhs, out);
            walk_expr_for_calls(rhs, out);
        }
        Stmt::If {
            cond,
            then_block,
            else_block,
            ..
        } => {
            walk_expr_for_calls(cond, out);
            walk_block_for_calls(then_block, out);
            if let Some(e) = else_block {
                walk_else_for_calls(e, out);
            }
        }
        Stmt::IfLet {
            expr,
            then_block,
            else_block,
            ..
        } => {
            walk_expr_for_calls(expr, out);
            walk_block_for_calls(then_block, out);
            if let Some(b) = else_block {
                walk_block_for_calls(b, out);
            }
        }
        Stmt::While { cond, body, .. } => {
            walk_expr_for_calls(cond, out);
            walk_block_for_calls(body, out);
        }
        Stmt::For {
            init,
            cond,
            step,
            body,
            ..
        } => {
            if let Some(i) = init {
                walk_for_init_for_calls(i, out);
            }
            if let Some(c) = cond {
                walk_expr_for_calls(c, out);
            }
            if let Some(s) = step {
                walk_for_step_for_calls(s, out);
            }
            walk_block_for_calls(body, out);
        }
        Stmt::Switch {
            expr,
            cases,
            default,
            ..
        } => {
            walk_expr_for_calls(expr, out);
            for c in cases {
                for s in &c.stmts {
                    walk_stmt_for_calls(s, out);
                }
            }
            if let Some(d) = default {
                for s in d {
                    walk_stmt_for_calls(s, out);
                }
            }
        }
        Stmt::Return { value, .. } => {
            if let Some(v) = value {
                walk_expr_for_calls(v, out);
            }
        }
        Stmt::Defer { body, .. } | Stmt::Unsafe { body, .. } => walk_block_for_calls(body, out),
        Stmt::Block(b) => walk_block_for_calls(b, out),
        Stmt::Expr { expr, .. } | Stmt::Discard { expr, .. } => walk_expr_for_calls(expr, out),
        Stmt::Break { .. } | Stmt::Continue { .. } => {}
    }
}

fn walk_else_for_calls(e: &ElseBranch, out: &mut Vec<String>) {
    match e {
        ElseBranch::ElseIf(s) => walk_stmt_for_calls(s, out),
        ElseBranch::Else(b) => walk_block_for_calls(b, out),
    }
}

fn walk_for_init_for_calls(fi: &ForInit, out: &mut Vec<String>) {
    match fi {
        ForInit::Let { init, .. } => walk_expr_for_calls(init, out),
        ForInit::Assign { lhs, rhs } => {
            walk_expr_for_calls(lhs, out);
            walk_expr_for_calls(rhs, out);
        }
        ForInit::Call(e) => walk_expr_for_calls(e, out),
    }
}

fn walk_for_step_for_calls(fs: &ForStep, out: &mut Vec<String>) {
    match fs {
        ForStep::Assign { lhs, rhs } => {
            walk_expr_for_calls(lhs, out);
            walk_expr_for_calls(rhs, out);
        }
        ForStep::Call(e) => walk_expr_for_calls(e, out),
    }
}

fn walk_expr_for_calls(expr: &Expr, out: &mut Vec<String>) {
    match expr {
        Expr::Call { callee, args, .. } => {
            if let Expr::Ident { name, .. } = callee.as_ref() {
                out.push(name.clone());
            }
            for a in args {
                walk_expr_for_calls(a, out);
            }
            walk_expr_for_calls(callee, out);
        }
        Expr::Binary { lhs, rhs, .. } => {
            walk_expr_for_calls(lhs, out);
            walk_expr_for_calls(rhs, out);
        }
        Expr::Unary { operand, .. } => walk_expr_for_calls(operand, out),
        Expr::Paren { inner, .. } => walk_expr_for_calls(inner, out),
        Expr::Field { base, .. } => walk_expr_for_calls(base, out),
        Expr::Addr { operand, .. } | Expr::AddrM { operand, .. } | Expr::Deref { operand, .. } => {
            walk_expr_for_calls(operand, out);
        }
        Expr::At { base, index, .. } => {
            walk_expr_for_calls(base, out);
            walk_expr_for_calls(index, out);
        }
        Expr::Cast { expr, .. } => walk_expr_for_calls(expr, out),
        Expr::Some { value, .. } | Expr::Ok { value, .. } | Expr::Err { value, .. } => {
            walk_expr_for_calls(value, out);
        }
        Expr::StructLit { fields, .. } => {
            for f in fields {
                walk_expr_for_calls(&f.value, out);
            }
        }
        Expr::Closure { body, .. } => walk_block_for_calls(body, out),
        Expr::Ident { .. }
        | Expr::IntLit { .. }
        | Expr::FloatLit { .. }
        | Expr::BoolLit { .. }
        | Expr::CStr { .. }
        | Expr::Bytes { .. }
        | Expr::None { .. }
        | Expr::SizeOf { .. } => {}
    }
}

fn transitive_banned_reach(
    start: &str,
    outgoing: &std::collections::HashMap<String, Vec<String>>,
) -> Vec<String> {
    use std::collections::HashSet;
    let mut visited: HashSet<String> = HashSet::new();
    let mut hits: HashSet<String> = HashSet::new();
    let mut queue: Vec<String> = vec![start.to_string()];
    while let Some(cur) = queue.pop() {
        if !visited.insert(cur.clone()) {
            continue;
        }
        if BANNED.iter().any(|b| *b == cur) {
            hits.insert(cur.clone());
            // Don't recurse into the allocator itself — we've
            // already proven the violation.
            continue;
        }
        if let Some(callees) = outgoing.get(&cur) {
            for c in callees {
                queue.push(c.clone());
            }
        }
    }
    let mut sorted: Vec<String> = hits.into_iter().collect();
    sorted.sort();
    sorted
}

#[cfg(test)]
mod tests {
    use crate::driver::compile;

    #[test]
    fn rejects_noalloc_function_that_calls_alloc() {
        // Direct: @noalloc fn that calls mem::alloc.
        let source = r#"
            use mem::alloc;
            @noalloc
            fn evil() -> rawm(u8) {
                return alloc(cast(usize, 16));
            }
            fn main() -> i32 {
                discard(evil());
                return 0;
            }
        "#;
        let result = compile(source, "evil.fc");
        assert!(
            result.is_err(),
            "expected @noalloc -> alloc to be rejected, got {:?}",
            result
        );
        let err = format!("{:?}", result.unwrap_err());
        assert!(
            err.contains("@noalloc"),
            "expected @noalloc diagnostic: {}",
            err
        );
    }

    #[test]
    fn accepts_noalloc_function_with_no_alloc_calls() {
        let source = r#"
            @noalloc
            fn pure_math(x: i32, y: i32) -> i32 {
                return (x * y) + x;
            }
            fn main() -> i32 {
                return pure_math(3, 4);
            }
        "#;
        let result = compile(source, "ok.fc");
        assert!(
            result.is_ok(),
            "clean @noalloc fn was rejected: {:?}",
            result
        );
    }
}
