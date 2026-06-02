//! v2.0 follow-up (I1) — call-site `@requires` discharge.
//!
//! For each call `f(arg0, arg1, …)` in the program, look up the
//! callee `f`'s `@requires` clauses, substitute the call's
//! arguments for `f`'s parameters in each clause, and run the
//! resulting expression through the existing 3-tier discharge
//! pipeline.
//!
//! ## Why this matters
//!
//! Stage 2.1's v1 SMT tier proves obligations that are *universally
//! true* over their parameters. A precondition like `@requires(x > 0)`
//! isn't universally true — it depends on what the caller passed —
//! so it fell to runtime. With call-site discharge, every call we
//! can statically analyze gets its callee's `@requires` re-checked
//! at the *call site* using the caller's context. A call like
//! `divisor_safe(5)` substitutes `x → 5`, the clause becomes
//! `5 > 0`, and tier-1 constant-folds it to `true`. Proven.
//!
//! ## What this v1 covers
//!
//! - Direct calls (`f(args)`) where `f` is a free function defined
//!   in the same compilation unit. Module-qualified callees
//!   (`mod::f`) are resolved via the `m::n` ident lookup that the
//!   AST already encodes through `Expr::Field`.
//! - Argument substitution into the supported expression subset
//!   (linear integer arithmetic + boolean combinators +
//!   comparisons).
//! - Caller's `@requires` clauses are fed to the SMT tier as
//!   assumptions, so a call like `safe_div(x, 2)` inside a function
//!   declared `@requires(x > 0)` can prove the callee's
//!   `@requires(divisor != 0)` when `divisor` substitutes to `2`,
//!   AND prove `@requires(value > 0)` when `value` substitutes to
//!   `x` under the caller's assumption that `x > 0`.
//!
//! ## Out of scope (deferred follow-up)
//!
//! - Method calls (`x.method()`) — the desugar pass converts those
//!   to free-function form before resolve, but the mangled name
//!   isn't trivially recoverable from the AST shape we're walking
//!   here.
//! - Calls inside `if` / `while` branches that we can't prove the
//!   branch condition for. v1 walks every body statement and
//!   discharges every call we see, but the path condition isn't
//!   added to the assumptions yet.
//! - Function-pointer calls. We need to know the callee statically
//!   to look up its `@requires`.

use std::collections::HashMap;

use crate::ast::{Block, Expr, File, Item, Stmt};

use super::{
    ContractObligation, DischargeConfig, DischargeReport, ObligationKind, ObligationParam, Status,
    Tier, expr_in_supported_subset, syntactic,
};

/// Drive the call-site discharge pass against a parsed `File`,
/// appending one obligation per call site / per callee-`@requires`
/// clause to `report`.
pub fn run(file: &File, cfg: &DischargeConfig, report: &mut DischargeReport) {
    // First pass: collect every fn's declared @requires plus param
    // names. This is the table we substitute into at every call site.
    let mut callees: HashMap<String, CalleeContract> = HashMap::new();
    collect_callees(&file.items, &[], &mut callees);

    let smt = if cfg.enable {
        Some(super::smt::SmtBackend::detect())
    } else {
        None
    };

    // Second pass: walk every fn body, find calls, discharge each.
    let mut module_path: Vec<String> = Vec::new();
    walk_items(
        &file.items,
        &mut module_path,
        &callees,
        cfg,
        smt.as_ref(),
        report,
    );
}

#[derive(Debug, Clone)]
struct CalleeContract {
    /// Callee parameter names, used for the substitution table when
    /// rewriting `@requires(x ...)` to use the call-site arg in
    /// position of `x`.
    param_names: Vec<String>,
    /// Callee `@requires` clauses to discharge at every call site.
    requires: Vec<Expr>,
}

fn collect_callees(
    items: &[Item],
    module_path: &[String],
    out: &mut HashMap<String, CalleeContract>,
) {
    for item in items {
        match item {
            Item::Fn(f) => {
                let key = mangled(module_path, &f.name);
                out.insert(
                    key,
                    CalleeContract {
                        param_names: f.params.iter().map(|p| p.name.clone()).collect(),
                        requires: f.requires.clone(),
                    },
                );
            }
            Item::Mod(m) => {
                if let Some(body) = &m.body {
                    let mut path = module_path.to_vec();
                    path.push(m.name.clone());
                    collect_callees(body, &path, out);
                }
            }
            _ => {}
        }
    }
}

fn walk_items(
    items: &[Item],
    module_path: &mut Vec<String>,
    callees: &HashMap<String, CalleeContract>,
    cfg: &DischargeConfig,
    smt: Option<&super::smt::SmtBackend>,
    report: &mut DischargeReport,
) {
    for item in items {
        match item {
            Item::Fn(f) => {
                let caller = CallerCtx {
                    name: mangled(module_path, &f.name),
                    params: f
                        .params
                        .iter()
                        .map(|p| ObligationParam {
                            name: p.name.clone(),
                            type_label: super::type_label_from(&p.ty),
                        })
                        .collect(),
                    assumptions: f.requires.clone(),
                    unsigned_param_names: caller_unsigned_param_names(f),
                };
                let mut counter: usize = 0;
                // N1: function-pointer bindings observed in this body.
                // `let f: fn(...) -> ... = some_fn;` records `f →
                // "some_fn"`. Subsequent `f(args)` call sites resolve
                // to the underlying free fn and route through the
                // same discharge path direct calls take.
                let mut fn_ptr_bindings: HashMap<String, String> = HashMap::new();
                walk_block(
                    &f.body,
                    &caller,
                    &mut counter,
                    callees,
                    &mut fn_ptr_bindings,
                    cfg,
                    smt,
                    report,
                );
            }
            Item::Mod(m) => {
                if let Some(body) = &m.body {
                    module_path.push(m.name.clone());
                    walk_items(body, module_path, callees, cfg, smt, report);
                    module_path.pop();
                }
            }
            _ => {}
        }
    }
}

#[derive(Debug, Clone)]
struct CallerCtx {
    name: String,
    params: Vec<ObligationParam>,
    assumptions: Vec<Expr>,
    /// I2: names of unsigned-typed caller params, propagated to
    /// each call-site obligation so the tier-1 nonneg pattern
    /// fires even when the substituted expression references the
    /// caller's params.
    unsigned_param_names: Vec<String>,
}

fn walk_block(
    block: &Block,
    caller: &CallerCtx,
    counter: &mut usize,
    callees: &HashMap<String, CalleeContract>,
    fn_ptrs: &mut HashMap<String, String>,
    cfg: &DischargeConfig,
    smt: Option<&super::smt::SmtBackend>,
    report: &mut DischargeReport,
) {
    for stmt in &block.stmts {
        walk_stmt(stmt, caller, counter, callees, fn_ptrs, cfg, smt, report);
    }
}

fn walk_stmt(
    stmt: &Stmt,
    caller: &CallerCtx,
    counter: &mut usize,
    callees: &HashMap<String, CalleeContract>,
    fn_ptrs: &mut HashMap<String, String>,
    cfg: &DischargeConfig,
    smt: Option<&super::smt::SmtBackend>,
    report: &mut DischargeReport,
) {
    match stmt {
        Stmt::Let { name, init, .. } => {
            // N1: when the RHS is an ident that names a known free
            // fn, record the binding so subsequent calls through
            // this local resolve to the underlying callee.
            if let Expr::Ident { name: rhs_name, .. } = init {
                if callees.contains_key(rhs_name) {
                    fn_ptrs.insert(name.clone(), rhs_name.clone());
                }
            }
            walk_expr(init, caller, counter, callees, fn_ptrs, cfg, smt, report);
        }
        Stmt::Assign { rhs, .. } => {
            walk_expr(rhs, caller, counter, callees, fn_ptrs, cfg, smt, report)
        }
        Stmt::Return { value: Some(e), .. } => {
            walk_expr(e, caller, counter, callees, fn_ptrs, cfg, smt, report)
        }
        Stmt::Expr { expr, .. } | Stmt::Discard { expr, .. } => {
            walk_expr(expr, caller, counter, callees, fn_ptrs, cfg, smt, report)
        }
        Stmt::Block(b) | Stmt::Unsafe { body: b, .. } | Stmt::Defer { body: b, .. } => {
            walk_block(b, caller, counter, callees, fn_ptrs, cfg, smt, report)
        }
        Stmt::While { cond, body, .. } => {
            walk_expr(cond, caller, counter, callees, fn_ptrs, cfg, smt, report);
            walk_block(body, caller, counter, callees, fn_ptrs, cfg, smt, report);
        }
        Stmt::If {
            cond,
            then_block,
            else_block,
            ..
        } => {
            walk_expr(cond, caller, counter, callees, fn_ptrs, cfg, smt, report);
            walk_block(
                then_block, caller, counter, callees, fn_ptrs, cfg, smt, report,
            );
            if let Some(eb) = else_block {
                match eb {
                    crate::ast::ElseBranch::Else(b) => {
                        walk_block(b, caller, counter, callees, fn_ptrs, cfg, smt, report)
                    }
                    crate::ast::ElseBranch::ElseIf(inner) => {
                        walk_stmt(inner, caller, counter, callees, fn_ptrs, cfg, smt, report)
                    }
                }
            }
        }
        _ => {} // For / Switch / IfLet / Break / Continue — v1 skips.
    }
}

fn walk_expr(
    expr: &Expr,
    caller: &CallerCtx,
    counter: &mut usize,
    callees: &HashMap<String, CalleeContract>,
    fn_ptrs: &mut HashMap<String, String>,
    cfg: &DischargeConfig,
    smt: Option<&super::smt::SmtBackend>,
    report: &mut DischargeReport,
) {
    if let Expr::Call { callee, args, .. } = expr {
        // N1: resolve the call's target through three lookups:
        //   1. Direct callee name (`f(args)` for a known free fn).
        //   2. Fn-pointer binding (`let f = some_fn; f(args)` →
        //      look up `f` in `fn_ptrs`, fall back to direct).
        //   3. Anything else (method calls, indirect-through-struct,
        //      unresolved idents) is skipped at this layer; post-mono
        //      it'll already be the rewritten free-fn form (per K1).
        if let Some(name) = callee_name(callee) {
            let resolved_name = fn_ptrs.get(&name).cloned().unwrap_or(name);
            if let Some(contract) = callees.get(&resolved_name) {
                discharge_call(
                    caller,
                    &resolved_name,
                    contract,
                    args,
                    counter,
                    cfg,
                    smt,
                    report,
                );
            }
        }
        // Recurse into args so nested calls discharge too.
        for a in args {
            walk_expr(a, caller, counter, callees, fn_ptrs, cfg, smt, report);
        }
    }

    // Recurse into compound expressions.
    match expr {
        Expr::Binary { lhs, rhs, .. } => {
            walk_expr(lhs, caller, counter, callees, fn_ptrs, cfg, smt, report);
            walk_expr(rhs, caller, counter, callees, fn_ptrs, cfg, smt, report);
        }
        Expr::Unary { operand, .. } | Expr::Paren { inner: operand, .. } => {
            walk_expr(operand, caller, counter, callees, fn_ptrs, cfg, smt, report);
        }
        _ => {}
    }
}

/// Pull a free-function name out of a call's callee expression.
/// `f(...)` → `Some("f")`. `mod::f(...)` shows up in the AST as
/// `Expr::Field` over an ident, so we encode it as `"mod::f"` for
/// the lookup table. Anything else (method calls, fn-ptr calls,
/// closures) returns `None` and the call is skipped.
fn callee_name(callee: &Expr) -> Option<String> {
    match callee {
        Expr::Ident { name, .. } => Some(name.clone()),
        Expr::Field { base, field, .. } => {
            if let Expr::Ident { name, .. } = base.as_ref() {
                Some(format!("{}::{}", name, field))
            } else {
                None
            }
        }
        _ => None,
    }
}

fn discharge_call(
    caller: &CallerCtx,
    callee_name: &str,
    callee: &CalleeContract,
    args: &[Expr],
    counter: &mut usize,
    cfg: &DischargeConfig,
    smt: Option<&super::smt::SmtBackend>,
    report: &mut DischargeReport,
) {
    // Arity mismatch (likely a generic specialization we don't yet
    // resolve) → skip. The standard typechecker catches real arity
    // errors elsewhere.
    if args.len() != callee.param_names.len() {
        return;
    }
    for (idx, requires) in callee.requires.iter().enumerate() {
        let call_id = *counter;
        *counter += 1;
        // Substitute every callee param ident with the corresponding
        // call-site argument expression.
        let substituted = substitute(requires, &callee.param_names, args);
        if !expr_in_supported_subset(&substituted) {
            // Substitution introduced an unsupported expression
            // (e.g. an arg was a function call). Skip silently —
            // the callee's own @requires runtime trap still fires.
            continue;
        }

        // Build a CallSite obligation. The function name encodes the
        // call site location: "<caller>::call<idx>->callee".
        let name = format!("{}::call{}->{}", caller.name, call_id, callee_name);
        let mut ob = ContractObligation {
            function: name,
            kind: ObligationKind::CallSite,
            index: idx,
            expr: substituted.clone(),
            params: caller.params.clone(),
            result_type: None,
            assumptions: caller.assumptions.clone(),
            body_result_expr: None,
            unsigned_param_names: caller.unsigned_param_names.clone(),
            status: Status::Runtime {
                reason: "no tier matched".to_string(),
            },
        };
        // Run tier-1 syntactic discharge directly.
        if syntactic::try_discharge(&ob) {
            ob.status = Status::Proven {
                tier: Tier::Syntactic,
            };
            report.obligations.push(ob);
            continue;
        }
        // Tier-2 SMT: only when --prove and z3 is available.
        if cfg.enable {
            if let Some(backend) = smt {
                match backend.try_discharge(&ob, cfg.smt_budget_ms, cfg.cache_root.as_deref()) {
                    super::smt::SmtResult::Proven => {
                        ob.status = Status::Proven { tier: Tier::Smt };
                    }
                    super::smt::SmtResult::Failed(_) => {
                        ob.status = Status::Runtime {
                            reason: format!(
                                "call site {}: caller context does not statically \
                                guarantee {}'s @requires[{}] — the callee's runtime \
                                trap still fires defensively.",
                                ob.function, callee_name, idx
                            ),
                        };
                    }
                    super::smt::SmtResult::Timeout => {
                        ob.status = Status::Unknown {
                            reason: format!(
                                "SMT timed out on call-site discharge for {}::{}",
                                callee_name, idx
                            ),
                        };
                    }
                    super::smt::SmtResult::Unsupported(reason) => {
                        ob.status = Status::Runtime { reason };
                    }
                }
            } else {
                ob.status = Status::Runtime {
                    reason: "z3 not on PATH — install z3 to enable SMT tier".to_string(),
                };
            }
        }
        report.obligations.push(ob);
    }
}

/// Replace every `Ident { name: P }` in `expr` with the corresponding
/// expression from `args` when `P` matches a name in `param_names`.
/// Returns a deep copy of the rewritten expression. Unsupported
/// constructs are returned unchanged — caller checks
/// `expr_in_supported_subset` afterward.
fn substitute(expr: &Expr, param_names: &[String], args: &[Expr]) -> Expr {
    match expr {
        Expr::Ident { name, .. } => {
            if let Some(pos) = param_names.iter().position(|p| p == name) {
                args[pos].clone()
            } else {
                expr.clone()
            }
        }
        Expr::Binary { op, lhs, rhs, span } => Expr::Binary {
            op: *op,
            lhs: Box::new(substitute(lhs, param_names, args)),
            rhs: Box::new(substitute(rhs, param_names, args)),
            span: span.clone(),
        },
        Expr::Unary { op, operand, span } => Expr::Unary {
            op: *op,
            operand: Box::new(substitute(operand, param_names, args)),
            span: span.clone(),
        },
        Expr::Paren { inner, span } => Expr::Paren {
            inner: Box::new(substitute(inner, param_names, args)),
            span: span.clone(),
        },
        _ => expr.clone(),
    }
}

fn mangled(module_path: &[String], name: &str) -> String {
    if module_path.is_empty() {
        name.to_string()
    } else {
        format!("{}::{}", module_path.join("::"), name)
    }
}

/// I2: caller-side unsigned-param-name extractor. Mirrors
/// `super::unsigned_param_names_for` but lives here to avoid a
/// circular `pub(crate)` re-export — each module owns its own
/// shape of "what's the caller context I care about".
fn caller_unsigned_param_names(f: &crate::ast::FnDecl) -> Vec<String> {
    use crate::ast::{PrimitiveType, TypeExpr};
    let mut out = Vec::new();
    for p in &f.params {
        if let TypeExpr::Primitive(prim) = &p.ty {
            if matches!(
                prim,
                PrimitiveType::U8
                    | PrimitiveType::U16
                    | PrimitiveType::U32
                    | PrimitiveType::U64
                    | PrimitiveType::Usize
            ) {
                out.push(p.name.clone());
            }
        }
    }
    out
}
