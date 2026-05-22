//! Capability lint — Stage 1.4 enforcement.
//!
//! Two policy checks today:
//!
//! 1. **Fabrication.** A sealed `Cap*` struct (or the `Caps` bundle)
//!    can only be constructed inside `mod caps`. Anywhere else, a
//!    struct literal `CapFsRead {}` is flagged as fabrication.
//!
//! 2. **`caps::init` is `main`-only.** The bundle-minting function
//!    can only be called from the top-level `main` (or from inside
//!    `mod caps` itself, which is how `caps::init` is defined). Any
//!    other call site would let arbitrary library code obtain the
//!    whole capability bundle — defeating the point of the system.
//!
//! What this catches:
//!
//!   ```ignore
//!   fn evil() -> CapFsRead {
//!       // ERROR (1): fabrication outside `mod caps`.
//!       return CapFsRead {};
//!   }
//!
//!   fn sneaky() -> Caps {
//!       // ERROR (2): caps::init outside `main` / `mod caps`.
//!       return caps::init();
//!   }
//!   ```
//!
//! What it allows:
//!
//!   - `caps::init()` inside `fn main`, top-level — the legitimate
//!     mint point.
//!   - `caps::init()` referenced inside `mod caps` — the function's
//!     own definition lives there.
//!   - Passing a cap value around as a function argument — that's
//!     normal type-checked argument passing.
//!
//! The pass runs in the driver between typecheck and mono. Errors
//! are reported through the standard `CompileError::multiple`
//! infrastructure so diagnostics render the same as any other
//! resolve / typecheck error.
//!
//! Known v1 limitation: the `init` check matches the *qualified*
//! callee name `caps::init`. A user who writes `use caps::init;
//! init()` calls the same function but the AST node carries the
//! bare name `init`. To plug this gap the lint scans the file's
//! `use` items and records which bare names alias `caps::init`,
//! then treats both spellings as the mint call.

use crate::ast::{Block, ElseBranch, Expr, File, FnDecl, ForInit, ForStep, Item, Stmt, UseItems};
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

/// Walker context. `inside_caps` controls fabrication; `init_allowed`
/// controls who may call `caps::init()`.
#[derive(Clone, Copy)]
struct Ctx<'a> {
    inside_caps: bool,
    init_allowed: bool,
    /// Bare names that have been imported as aliases for
    /// `caps::init`. Always at least empty.
    init_aliases: &'a [String],
}

/// Run the capability lint against `file`.
pub fn check_caps(file: &File, source: &str) -> Result<(), CompileError> {
    let mut errors: Vec<CompileError> = Vec::new();
    let aliases = collect_init_aliases(&file.items);
    walk_items(&file.items, &[], &aliases, &mut errors, source);
    if errors.is_empty() {
        Ok(())
    } else {
        Err(CompileError::multiple(errors))
    }
}

/// Scan the file's `use` items for anything that imports
/// `caps::init` and record the bare name(s) it can be called by.
/// Today the canonical form is `use caps::init;` which binds `init`
/// at the call site; `use caps::{init};` is handled the same way.
fn collect_init_aliases(items: &[Item]) -> Vec<String> {
    let mut aliases: Vec<String> = Vec::new();
    for item in items {
        if let Item::Use(u) = item
            && u.path.first().map(String::as_str) == Some("caps")
        {
            match &u.items {
                UseItems::Single(name) if name == "init" => {
                    aliases.push("init".to_string());
                }
                UseItems::Multiple(names) => {
                    if names.iter().any(|n| n == "init") {
                        aliases.push("init".to_string());
                    }
                }
                _ => {}
            }
        }
    }
    aliases
}

fn walk_items(
    items: &[Item],
    module_path: &[String],
    init_aliases: &[String],
    errors: &mut Vec<CompileError>,
    source: &str,
) {
    for item in items {
        match item {
            Item::Fn(f) => walk_fn(f, module_path, init_aliases, errors, source),
            Item::Mod(m) => {
                if let Some(body) = &m.body {
                    let mut new_path = module_path.to_vec();
                    new_path.push(m.name.clone());
                    walk_items(body, &new_path, init_aliases, errors, source);
                }
            }
            Item::Impl(block) => {
                for method in &block.methods {
                    walk_fn(method, module_path, init_aliases, errors, source);
                }
            }
            _ => {}
        }
    }
}

fn walk_fn(
    f: &FnDecl,
    module_path: &[String],
    init_aliases: &[String],
    errors: &mut Vec<CompileError>,
    source: &str,
) {
    let inside_caps = is_inside_caps(module_path);
    let is_root_main = module_path.is_empty() && f.name == "main";
    let ctx = Ctx {
        inside_caps,
        init_allowed: inside_caps || is_root_main,
        init_aliases,
    };
    walk_block(&f.body, ctx, errors, source);
    for req in &f.requires {
        walk_expr(req, ctx, errors, source);
    }
}

fn is_inside_caps(module_path: &[String]) -> bool {
    module_path.iter().any(|s| s == "caps")
}

fn walk_block(block: &Block, ctx: Ctx, errors: &mut Vec<CompileError>, source: &str) {
    for s in &block.stmts {
        walk_stmt(s, ctx, errors, source);
    }
}

fn walk_stmt(stmt: &Stmt, ctx: Ctx, errors: &mut Vec<CompileError>, source: &str) {
    match stmt {
        Stmt::Let { init, .. } => walk_expr(init, ctx, errors, source),
        Stmt::Assign { lhs, rhs, .. } => {
            walk_expr(lhs, ctx, errors, source);
            walk_expr(rhs, ctx, errors, source);
        }
        Stmt::If {
            cond,
            then_block,
            else_block,
            ..
        } => {
            walk_expr(cond, ctx, errors, source);
            walk_block(then_block, ctx, errors, source);
            if let Some(e) = else_block {
                walk_else(e, ctx, errors, source);
            }
        }
        Stmt::IfLet {
            expr,
            then_block,
            else_block,
            ..
        } => {
            walk_expr(expr, ctx, errors, source);
            walk_block(then_block, ctx, errors, source);
            if let Some(b) = else_block {
                walk_block(b, ctx, errors, source);
            }
        }
        Stmt::While { cond, body, .. } => {
            walk_expr(cond, ctx, errors, source);
            walk_block(body, ctx, errors, source);
        }
        Stmt::For {
            init,
            cond,
            step,
            body,
            ..
        } => {
            if let Some(i) = init {
                walk_for_init(i, ctx, errors, source);
            }
            if let Some(c) = cond {
                walk_expr(c, ctx, errors, source);
            }
            if let Some(s) = step {
                walk_for_step(s, ctx, errors, source);
            }
            walk_block(body, ctx, errors, source);
        }
        Stmt::Switch {
            expr,
            cases,
            default,
            ..
        } => {
            walk_expr(expr, ctx, errors, source);
            for c in cases {
                for s in &c.stmts {
                    walk_stmt(s, ctx, errors, source);
                }
            }
            if let Some(d) = default {
                for s in d {
                    walk_stmt(s, ctx, errors, source);
                }
            }
        }
        Stmt::Return { value, .. } => {
            if let Some(v) = value {
                walk_expr(v, ctx, errors, source);
            }
        }
        Stmt::Defer { body, .. } | Stmt::Unsafe { body, .. } => {
            walk_block(body, ctx, errors, source);
        }
        Stmt::Block(b) => walk_block(b, ctx, errors, source),
        Stmt::Expr { expr, .. } | Stmt::Discard { expr, .. } => {
            walk_expr(expr, ctx, errors, source);
        }
        Stmt::Break { .. } | Stmt::Continue { .. } => {}
    }
}

fn walk_else(e: &ElseBranch, ctx: Ctx, errors: &mut Vec<CompileError>, source: &str) {
    match e {
        ElseBranch::ElseIf(s) => walk_stmt(s, ctx, errors, source),
        ElseBranch::Else(b) => walk_block(b, ctx, errors, source),
    }
}

fn walk_for_init(fi: &ForInit, ctx: Ctx, errors: &mut Vec<CompileError>, source: &str) {
    match fi {
        ForInit::Let { init, .. } => walk_expr(init, ctx, errors, source),
        ForInit::Assign { lhs, rhs } => {
            walk_expr(lhs, ctx, errors, source);
            walk_expr(rhs, ctx, errors, source);
        }
        ForInit::Call(e) => walk_expr(e, ctx, errors, source),
    }
}

fn walk_for_step(fs: &ForStep, ctx: Ctx, errors: &mut Vec<CompileError>, source: &str) {
    match fs {
        ForStep::Assign { lhs, rhs } => {
            walk_expr(lhs, ctx, errors, source);
            walk_expr(rhs, ctx, errors, source);
        }
        ForStep::Call(e) => walk_expr(e, ctx, errors, source),
    }
}

fn walk_expr(expr: &Expr, ctx: Ctx, errors: &mut Vec<CompileError>, source: &str) {
    match expr {
        Expr::StructLit { name, fields, span } => {
            if !ctx.inside_caps && SEALED_CAPS.iter().any(|s| *s == name) {
                errors.push(report_fabrication(name, span, source));
            }
            for f in fields {
                walk_expr(&f.value, ctx, errors, source);
            }
        }
        Expr::Call { callee, args, span } => {
            if let Expr::Ident { name, .. } = callee.as_ref()
                && is_init_call(name, ctx.init_aliases)
                && !ctx.init_allowed
            {
                errors.push(report_init_misuse(span, source));
            }
            walk_expr(callee, ctx, errors, source);
            for a in args {
                walk_expr(a, ctx, errors, source);
            }
        }
        Expr::Binary { lhs, rhs, .. } => {
            walk_expr(lhs, ctx, errors, source);
            walk_expr(rhs, ctx, errors, source);
        }
        Expr::Unary { operand, .. } => walk_expr(operand, ctx, errors, source),
        Expr::Paren { inner, .. } => walk_expr(inner, ctx, errors, source),
        Expr::Field { base, .. } => walk_expr(base, ctx, errors, source),
        Expr::Addr { operand, .. } | Expr::AddrM { operand, .. } | Expr::Deref { operand, .. } => {
            walk_expr(operand, ctx, errors, source);
        }
        Expr::At { base, index, .. } => {
            walk_expr(base, ctx, errors, source);
            walk_expr(index, ctx, errors, source);
        }
        Expr::Cast { expr, .. } => walk_expr(expr, ctx, errors, source),
        Expr::Some { value, .. } | Expr::Ok { value, .. } | Expr::Err { value, .. } => {
            walk_expr(value, ctx, errors, source);
        }
        Expr::Closure { body, .. } => walk_block(body, ctx, errors, source),
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

fn is_init_call(name: &str, init_aliases: &[String]) -> bool {
    name == "caps::init" || init_aliases.iter().any(|a| a == name)
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

fn report_init_misuse(span: &Span, source: &str) -> CompileError {
    CompileError::resolve(
        "capability misuse: `caps::init()` is `main`-only. Receive caps as function arguments instead of minting them — any other call site would let arbitrary code mint the whole bundle.".to_string(),
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

    #[test]
    fn rejects_caps_init_outside_main() {
        // A user library calling `caps::init()` from a non-main
        // function bypasses the fabrication check by going through
        // the legitimate mint helper. The lint must catch this.
        let source = r#"
            fn sneaky() -> Caps {
                return caps::init();
            }
            fn main() -> i32 {
                let c: Caps = sneaky();
                discard(c);
                return 0;
            }
        "#;
        let result = compile(source, "sneaky.fc");
        assert!(
            result.is_err(),
            "expected caps::init misuse to be rejected, got: {:?}",
            result
        );
        let err = format!("{:?}", result.unwrap_err());
        assert!(
            err.contains("`main`-only") || err.contains("capability misuse"),
            "expected caps::init diagnostic, got: {}",
            err
        );
    }

    #[test]
    fn rejects_init_alias_outside_main() {
        // The same attack via `use caps::init`. Calling bare `init()`
        // from a non-main function must also fail — the alias
        // scanner records the bare-name spelling.
        let source = r#"
            use caps::init;
            fn sneaky() -> Caps {
                return init();
            }
            fn main() -> i32 {
                let c: Caps = sneaky();
                discard(c);
                return 0;
            }
        "#;
        let result = compile(source, "alias.fc");
        assert!(
            result.is_err(),
            "expected aliased init misuse to be rejected, got: {:?}",
            result
        );
    }
}
