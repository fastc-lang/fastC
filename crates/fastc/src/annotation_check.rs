//! v1.3 annotation enforcement.
//!
//! Enforces the two structured annotations that have static-checkable
//! semantics:
//!
//! - `@panics(never)` — the function must not reach a trap-emitting
//!   callee via the transitive call graph. Banned set: `fc_trap`,
//!   `panic`, `abort`, `exit` (plus their qualified prelude forms).
//!   v1.x cannot statically track implicit traps from overflow /
//!   bounds-check failures inside the function body — only explicit
//!   calls. That's a documented limitation; `@panics(never)` here
//!   means "this function doesn't intentionally trap", not "no
//!   path can fault".
//!
//! - `@purity(pure)` — the function must be free of allocation,
//!   global mutable state reads, and observable I/O. v1.x checks
//!   the call-set conservatively: no calls to the allocator names
//!   `@noalloc` already bans, plus no calls into `io::*` / `log::*`
//!   / `http::*` / `fs::*` / `net::*`.
//!
//! The other v1.3 annotations (`@mem(arena=...)`, `@panics(always)`,
//! `@panics(on=...)`, `@purity(effect|io)`, `@complexity(O(...))`) are
//! documentation-only in v1.x. They flow through `fastc explain`
//! JSON, `caps.json`, and `cert-report` so downstream auditors can
//! consume them, but they don't gate compilation.

use crate::ast::{
    Block, ElseBranch, Expr, File, FnDecl, ForInit, ForStep, Item, PurityLevel, Stmt,
};
use crate::diag::CompileError;
use std::collections::{HashMap, HashSet};

/// Names that, when reached transitively from a `@panics(never)` fn,
/// constitute a violation. Both bare and qualified prelude forms are
/// listed because callers may write either.
const PANIC_TRAP_NAMES: &[&str] = &[
    "fc_trap",
    "panic",
    "abort",
    "exit",
    "fc_panic",
    "core::panic",
    "core::abort",
];

/// Names that a `@purity(pure)` function cannot reach. The allocator
/// banned-list is shared with `@noalloc`; the I/O banned-list covers
/// the prelude's user-facing observable-effect surface.
const PURITY_BANNED: &[&str] = &[
    // Allocator (mirror of noalloc_check::BANNED)
    "alloc",
    "resize",
    "free_bytes",
    "mem::alloc",
    "mem::resize",
    "mem::free_bytes",
    "malloc",
    "realloc",
    "free",
    "mem::malloc",
    "mem::realloc",
    "mem::free",
    // I/O
    "println",
    "put_char",
    "print_int",
    "io::println",
    "io::put_char",
    "io::print_int",
    // Structured logging
    "log::debug",
    "log::info",
    "log::warn",
    "log::error",
    "log::kv_int",
    "log::kv_str",
    // HTTP, fs, net, env, time, rand — any cap-using prelude surface
    "http::get_status",
    "fs::exists",
    "fs::size_bytes",
    "env::get",
    "time::now",
    "rand::seed",
    "rand::next_u32",
];

pub fn check_annotations(file: &File, source: &str) -> Result<(), CompileError> {
    let mut all_fns: Vec<(Vec<String>, FnDecl)> = Vec::new();
    collect_fns(&file.items, &[], &mut all_fns);

    // Build the outgoing-calls map once; both checks reuse it.
    let mut outgoing: HashMap<String, Vec<String>> = HashMap::new();
    for (_path, f) in &all_fns {
        let mut calls = Vec::new();
        walk_block_for_calls(&f.body, &mut calls);
        outgoing.insert(f.name.clone(), calls);
    }

    let mut errors: Vec<CompileError> = Vec::new();

    for (_path, f) in &all_fns {
        // @panics(never) check
        if matches!(f.panics, Some(crate::ast::PanicsAnnot::Never)) {
            let trap_reach = transitive_reach(&f.name, &outgoing, PANIC_TRAP_NAMES);
            for reached in trap_reach {
                errors.push(CompileError::resolve(
                    format!(
                        "@panics(never) function '{}' reaches '{}' (transitive call). \
                         Either drop the annotation or refactor to avoid the trap path.",
                        f.name, reached
                    ),
                    f.span.clone(),
                    source,
                ));
            }
        }

        // @purity(pure) check
        if matches!(f.purity, Some(PurityLevel::Pure)) {
            let impure_reach = transitive_reach(&f.name, &outgoing, PURITY_BANNED);
            for reached in impure_reach {
                errors.push(CompileError::resolve(
                    format!(
                        "@purity(pure) function '{}' reaches '{}' (transitive call). \
                         Pure functions cannot allocate, log, or perform I/O. \
                         Drop the annotation or refactor to remove the side effect.",
                        f.name, reached
                    ),
                    f.span.clone(),
                    source,
                ));
            }
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

fn transitive_reach(
    start: &str,
    outgoing: &HashMap<String, Vec<String>>,
    banned: &[&str],
) -> Vec<String> {
    let mut visited: HashSet<String> = HashSet::new();
    let mut hits: HashSet<String> = HashSet::new();
    let mut queue: Vec<String> = vec![start.to_string()];
    while let Some(cur) = queue.pop() {
        if !visited.insert(cur.clone()) {
            continue;
        }
        if banned.iter().any(|b| *b == cur) {
            hits.insert(cur.clone());
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
