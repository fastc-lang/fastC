//! Capability lint — Stage 1.4 enforcement (minimal).
//!
//! Forbids fabricating capability values outside the prelude's
//! `mod caps`. Today the sealed list is hardcoded (every `Cap*`
//! struct plus the `Caps` bundle); a future pass will walk a
//! `@sealed` attribute declared in source instead.
//!
//! What this catches:
//!
//!   ```ignore
//!   fn evil() -> CapFsRead {
//!       // ERROR: capability fabrication outside `mod caps`.
//!       return CapFsRead {};
//!   }
//!   ```
//!
//! What it allows:
//!
//!   - `caps::init()` inside `mod caps` — that's where caps are
//!     legitimately minted.
//!   - Passing a cap value around as a function argument — that's
//!     normal type-checked argument passing.
//!
//! The pass runs in the driver between typecheck and mono. Errors
//! are reported through the standard `CompileError::multiple`
//! infrastructure so diagnostics render the same as any other
//! resolve / typecheck error.

use crate::ast::{Block, ElseBranch, Expr, File, FnDecl, ForInit, ForStep, Item, Stmt};
use crate::diag::CompileError;
use crate::lexer::Span;

/// Every struct name in this list can only be constructed by code
/// living inside the `caps` module. Anywhere else the construction
/// is flagged as capability fabrication.
const SEALED_CAPS: &[&str] = &[
    "CapFsRead",
    "CapFsWrite",
    "CapNetConnect",
    "CapNetListen",
    "CapProcSpawn",
    "CapTimeRead",
    "CapRand",
    "CapEnvRead",
    "Caps",
];

/// Run the capability lint against `file`. Returns Err with one or
/// more diagnostics when any sealed capability is constructed
/// outside `mod caps`.
pub fn check_caps(file: &File, source: &str) -> Result<(), CompileError> {
    let mut errors: Vec<CompileError> = Vec::new();
    walk_items(&file.items, &[], &mut errors, source);
    if errors.is_empty() {
        Ok(())
    } else {
        Err(CompileError::multiple(errors))
    }
}

fn walk_items(
    items: &[Item],
    module_path: &[String],
    errors: &mut Vec<CompileError>,
    source: &str,
) {
    for item in items {
        match item {
            Item::Fn(f) => walk_fn(f, module_path, errors, source),
            Item::Mod(m) => {
                if let Some(body) = &m.body {
                    let mut new_path = module_path.to_vec();
                    new_path.push(m.name.clone());
                    walk_items(body, &new_path, errors, source);
                }
            }
            Item::Impl(block) => {
                // Methods inside an impl block also get walked. The
                // module path here is the file scope unless we're
                // inside a `mod ... { impl ... }`, which the recursive
                // call above already threads through.
                for method in &block.methods {
                    walk_fn(method, module_path, errors, source);
                }
            }
            _ => {}
        }
    }
}

fn walk_fn(f: &FnDecl, module_path: &[String], errors: &mut Vec<CompileError>, source: &str) {
    let inside_caps = is_inside_caps(module_path);
    walk_block(&f.body, inside_caps, errors, source);
    for req in &f.requires {
        walk_expr(req, inside_caps, errors, source);
    }
}

fn is_inside_caps(module_path: &[String]) -> bool {
    module_path.iter().any(|s| s == "caps")
}

fn walk_block(block: &Block, inside_caps: bool, errors: &mut Vec<CompileError>, source: &str) {
    for s in &block.stmts {
        walk_stmt(s, inside_caps, errors, source);
    }
}

fn walk_stmt(stmt: &Stmt, inside_caps: bool, errors: &mut Vec<CompileError>, source: &str) {
    match stmt {
        Stmt::Let { init, .. } => walk_expr(init, inside_caps, errors, source),
        Stmt::Assign { lhs, rhs, .. } => {
            walk_expr(lhs, inside_caps, errors, source);
            walk_expr(rhs, inside_caps, errors, source);
        }
        Stmt::If {
            cond,
            then_block,
            else_block,
            ..
        } => {
            walk_expr(cond, inside_caps, errors, source);
            walk_block(then_block, inside_caps, errors, source);
            if let Some(e) = else_block {
                walk_else(e, inside_caps, errors, source);
            }
        }
        Stmt::IfLet {
            expr,
            then_block,
            else_block,
            ..
        } => {
            walk_expr(expr, inside_caps, errors, source);
            walk_block(then_block, inside_caps, errors, source);
            if let Some(b) = else_block {
                walk_block(b, inside_caps, errors, source);
            }
        }
        Stmt::While { cond, body, .. } => {
            walk_expr(cond, inside_caps, errors, source);
            walk_block(body, inside_caps, errors, source);
        }
        Stmt::For {
            init,
            cond,
            step,
            body,
            ..
        } => {
            if let Some(i) = init {
                walk_for_init(i, inside_caps, errors, source);
            }
            if let Some(c) = cond {
                walk_expr(c, inside_caps, errors, source);
            }
            if let Some(s) = step {
                walk_for_step(s, inside_caps, errors, source);
            }
            walk_block(body, inside_caps, errors, source);
        }
        Stmt::Switch {
            expr,
            cases,
            default,
            ..
        } => {
            walk_expr(expr, inside_caps, errors, source);
            for c in cases {
                for s in &c.stmts {
                    walk_stmt(s, inside_caps, errors, source);
                }
            }
            if let Some(d) = default {
                for s in d {
                    walk_stmt(s, inside_caps, errors, source);
                }
            }
        }
        Stmt::Return { value, .. } => {
            if let Some(v) = value {
                walk_expr(v, inside_caps, errors, source);
            }
        }
        Stmt::Defer { body, .. } | Stmt::Unsafe { body, .. } => {
            walk_block(body, inside_caps, errors, source);
        }
        Stmt::Block(b) => walk_block(b, inside_caps, errors, source),
        Stmt::Expr { expr, .. } | Stmt::Discard { expr, .. } => {
            walk_expr(expr, inside_caps, errors, source);
        }
        Stmt::Break { .. } | Stmt::Continue { .. } => {}
    }
}

fn walk_else(e: &ElseBranch, inside_caps: bool, errors: &mut Vec<CompileError>, source: &str) {
    match e {
        ElseBranch::ElseIf(s) => walk_stmt(s, inside_caps, errors, source),
        ElseBranch::Else(b) => walk_block(b, inside_caps, errors, source),
    }
}

fn walk_for_init(fi: &ForInit, inside_caps: bool, errors: &mut Vec<CompileError>, source: &str) {
    match fi {
        ForInit::Let { init, .. } => walk_expr(init, inside_caps, errors, source),
        ForInit::Assign { lhs, rhs } => {
            walk_expr(lhs, inside_caps, errors, source);
            walk_expr(rhs, inside_caps, errors, source);
        }
        ForInit::Call(e) => walk_expr(e, inside_caps, errors, source),
    }
}

fn walk_for_step(fs: &ForStep, inside_caps: bool, errors: &mut Vec<CompileError>, source: &str) {
    match fs {
        ForStep::Assign { lhs, rhs } => {
            walk_expr(lhs, inside_caps, errors, source);
            walk_expr(rhs, inside_caps, errors, source);
        }
        ForStep::Call(e) => walk_expr(e, inside_caps, errors, source),
    }
}

fn walk_expr(expr: &Expr, inside_caps: bool, errors: &mut Vec<CompileError>, source: &str) {
    match expr {
        Expr::StructLit { name, fields, span } => {
            if !inside_caps && SEALED_CAPS.iter().any(|s| *s == name) {
                errors.push(report_fabrication(name, span, source));
            }
            for f in fields {
                walk_expr(&f.value, inside_caps, errors, source);
            }
        }
        Expr::Call { callee, args, .. } => {
            walk_expr(callee, inside_caps, errors, source);
            for a in args {
                walk_expr(a, inside_caps, errors, source);
            }
        }
        Expr::Binary { lhs, rhs, .. } => {
            walk_expr(lhs, inside_caps, errors, source);
            walk_expr(rhs, inside_caps, errors, source);
        }
        Expr::Unary { operand, .. } => walk_expr(operand, inside_caps, errors, source),
        Expr::Paren { inner, .. } => walk_expr(inner, inside_caps, errors, source),
        Expr::Field { base, .. } => walk_expr(base, inside_caps, errors, source),
        Expr::Addr { operand, .. } | Expr::AddrM { operand, .. } | Expr::Deref { operand, .. } => {
            walk_expr(operand, inside_caps, errors, source);
        }
        Expr::At { base, index, .. } => {
            walk_expr(base, inside_caps, errors, source);
            walk_expr(index, inside_caps, errors, source);
        }
        Expr::Cast { expr, .. } => walk_expr(expr, inside_caps, errors, source),
        Expr::Some { value, .. } | Expr::Ok { value, .. } | Expr::Err { value, .. } => {
            walk_expr(value, inside_caps, errors, source);
        }
        Expr::Closure { body, .. } => walk_block(body, inside_caps, errors, source),
        Expr::IntLit { .. }
        | Expr::FloatLit { .. }
        | Expr::BoolLit { .. }
        | Expr::Ident { .. }
        | Expr::CStr { .. }
        | Expr::Bytes { .. }
        | Expr::None { .. }
        | Expr::SizeOf { .. } => {}
    }
}

fn report_fabrication(name: &str, span: &Span, source: &str) -> CompileError {
    CompileError::resolve(
        format!(
            "capability fabrication: '{}' can only be constructed inside `mod caps`. Receive it as a function argument instead, or call `caps::init()` from `main`.",
            name
        ),
        span.clone(),
        source,
    )
}

#[cfg(test)]
mod tests {
    use crate::driver::compile;

    #[test]
    fn rejects_user_capability_fabrication() {
        // User code outside `mod caps` constructing a sealed Cap is
        // the canonical capability-fabrication attack. The lint
        // must reject this.
        let source = r#"
            fn evil() -> CapFsRead {
                return CapFsRead {};
            }

            fn main() -> i32 {
                let c: CapFsRead = evil();
                discard(c);
                return 0;
            }
        "#;
        let result = compile(source, "evil.fc");
        assert!(
            result.is_err(),
            "expected fabrication to be rejected, got: {:?}",
            result
        );
        let err = format!("{:?}", result.unwrap_err());
        assert!(
            err.contains("capability fabrication"),
            "expected fabrication error, got: {}",
            err
        );
    }

    #[test]
    fn accepts_caps_init_in_main() {
        // The standard use pattern: main mints caps and passes them
        // through. No fabrication anywhere outside `mod caps`.
        let source = r#"
            use caps::init;
            fn count_files(c: ref(CapFsRead)) -> i32 {
                discard(c);
                return 0;
            }
            fn main() -> i32 {
                let caps: Caps = init();
                return count_files(addr(caps.fs_read));
            }
        "#;
        let result = compile(source, "ok.fc");
        assert!(result.is_ok(), "legitimate cap use rejected: {:?}", result);
    }
}
