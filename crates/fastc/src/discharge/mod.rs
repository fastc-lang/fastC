//! Stage 2.1 — SMT contract discharge.
//!
//! Walks every fastC function, collects its `@requires` / `@ensures`
//! clauses as `ContractObligation`s, and runs each one through a
//! three-tier pipeline:
//!
//! 1. **Syntactic** (always on, cheap). A pattern-matching pass over
//!    the AST that proves obvious cases — `@requires(x > 0)` where
//!    the call site passes a literal positive, `@ensures(result >= 0)`
//!    where the body returns a constant, that kind of thing. No SMT
//!    solver involved, no cost.
//!
//! 2. **SMT** (opt-in via `--prove`). Encodes the obligation into
//!    SMT-LIB and shells out to `z3 -smt2 -in` with a per-obligation
//!    time budget (default 500 ms). Z3-not-on-PATH degrades to a
//!    warning + tier-3 fallback, mirroring the cosign-in-supply-chain
//!    integration so a fastC install is never blocked by a missing
//!    external tool.
//!
//! 3. **Runtime** (the existing stage-1.5 behavior). Anything tier-1
//!    and tier-2 couldn't prove keeps its `if (!cond) fc_trap()`
//!    guard in the lowered C. The proof gap stays observable.
//!
//! ## Why a separate module
//!
//! The lower pass is where the runtime traps are emitted today. Doing
//! discharge inside lower would entangle pre-compile-time analysis
//! with code generation. Keeping discharge as its own pass — running
//! between typecheck and lower — means:
//!
//! - The lower pass receives a `DischargeReport` and elides traps for
//!   `Status::Proven` obligations. The check stays in source; the
//!   trap leaves the binary.
//! - The `discharge.json` artifact reflects exactly one snapshot per
//!   build, comparable across CI runs.
//! - SMT-related code stays out of the hot compile path when
//!   `--no-prove` is set (the default for `fastc check`).
//!
//! ## What this v1 covers
//!
//! - Linear integer arithmetic (`+`, `-`, `*` against literal,
//!   `<`, `<=`, `>`, `>=`, `==`, `!=`).
//! - Boolean combinators (`&&`, `||`, `!`).
//! - Function-parameter quantification (every requires/ensures is
//!   universal over the function's parameters).
//! - `result` identifier in `@ensures` for the return value.
//!
//! ## Out of scope (deferred to a later v2.1.x)
//!
//! - Nonlinear arithmetic (multiplication of two variables).
//! - Floating-point arithmetic (Z3 supports it but the encoding
//!   adds surface area we don't need for v1).
//! - User-defined predicates / pure functions in obligations.
//! - Heap reasoning (separation logic).
//! - Quantifier alternation beyond the implicit-universal envelope.

use std::collections::BTreeMap;

use crate::ast::{BinOp, Expr, File, FnDecl, Item, UnaryOp};

mod cache;
mod callsite;
mod smt;
mod syntactic;

#[cfg(test)]
mod tests;

pub use smt::SmtBackend;

/// Configuration controlling how the discharge pass behaves.
#[derive(Debug, Clone)]
pub struct DischargeConfig {
    /// When false, every obligation is forced to `Runtime` — the
    /// pass still collects and reports, but it doesn't try to prove
    /// anything. This is the default for `fastc check` so the
    /// inner-loop feedback stays fast.
    pub enable: bool,
    /// Per-obligation budget for the SMT tier, in milliseconds.
    /// Z3 is called with `set-option :timeout <ms>` plus a process
    /// `timeout()` ceiling at 2× this value as a hard safety bound.
    pub smt_budget_ms: u64,
    /// Project root for the on-disk SMT cache at
    /// `<root>/.fastc/cache/discharge/`. `None` skips caching —
    /// useful for tests that don't want side-effects on disk and
    /// for one-shot `fastc compile <file.fc>` invocations that
    /// don't live inside a project.
    pub cache_root: Option<std::path::PathBuf>,
}

impl Default for DischargeConfig {
    fn default() -> Self {
        Self {
            enable: false,
            smt_budget_ms: 500,
            cache_root: None,
        }
    }
}

/// Status of a single contract obligation after running the pipeline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Status {
    /// Statically proven (tier-1 syntactic or tier-2 SMT). The lower
    /// pass should elide the runtime trap for this obligation.
    Proven { tier: Tier },
    /// Couldn't prove — the runtime trap stays in the lowered code.
    /// This is the safe default; a "miss" never weakens the program.
    Runtime { reason: String },
    /// SMT timed out / returned `unknown`. Distinguishable from
    /// `Runtime` for reporting but behaves the same way at lower
    /// time (trap stays in).
    Unknown { reason: String },
}

/// Which tier discharged this obligation. Used in the report so
/// users can see how much work came from cheap syntactic patterns
/// vs the SMT solver vs left for runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tier {
    Syntactic,
    Smt,
}

/// Which contract clause an obligation came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObligationKind {
    Requires,
    Ensures,
    /// I1 (v2.0): a call site re-checking a callee's `@requires`
    /// with the caller's argument substitutions + assumptions.
    /// Distinct from `Requires` so the report can split "did the
    /// caller satisfy the precondition?" from "is this precondition
    /// universally true?".
    CallSite,
}

/// A single thing the SMT pipeline tries to prove.
#[derive(Debug, Clone)]
pub struct ContractObligation {
    /// Owning function name (mangled module-qualified form).
    pub function: String,
    /// `requires` or `ensures`.
    pub kind: ObligationKind,
    /// Zero-based index inside the owner function's clause list.
    /// `@requires[0]`, `@requires[1]`, … so the lower pass can match
    /// up obligations to specific lowered checks.
    pub index: usize,
    /// The boolean expression that must hold.
    pub expr: Expr,
    /// The function's typed parameter list, captured at discharge
    /// time so the SMT encoder doesn't have to re-derive types.
    pub params: Vec<ObligationParam>,
    /// For `ensures`, the return type so `result` can be encoded.
    /// `None` for `requires` and for `ensures` on `void` returns.
    pub result_type: Option<String>,
    /// Stage-2.1 follow-up (H1): for `@ensures` obligations, the
    /// function's `@requires` clauses are fed to Z3 as assumptions
    /// — the precondition strengthens what we can prove about the
    /// postcondition. Empty for `@requires` obligations.
    pub assumptions: Vec<Expr>,
    /// Stage-2.1 follow-up (H1): for `@ensures` obligations on a
    /// straight-line function body (single `return <expr>` over
    /// supported operators), the expression for `result`. Z3 gets
    /// `(assert (= result <body_expr>))` so the obligation can
    /// reference body-computed values. `None` for functions whose
    /// body is too complex to model (loops, branches, calls to
    /// user fns, pointer ops, etc).
    pub body_result_expr: Option<Expr>,
    /// I2 (v2.0): names of params whose AST type is unsigned
    /// (`u8` / `u16` / `u32` / `u64` / `usize`). The syntactic
    /// discharger uses this to short-circuit `@requires(n >= 0)`
    /// patterns without consulting the SMT tier — unsigned
    /// integers are nonnegative by construction.
    pub unsigned_param_names: Vec<String>,
    /// Result populated by the pipeline.
    pub status: Status,
}

#[derive(Debug, Clone)]
pub struct ObligationParam {
    pub name: String,
    /// Coarse type label — "i32" / "i64" / "bool" / "ptr" / "unknown".
    /// SMT encoding maps these to Z3 sorts. Unknown types disqualify
    /// the obligation from the SMT tier (it falls to runtime).
    pub type_label: String,
}

/// Top-level discharge result for a whole compilation unit.
#[derive(Debug, Clone, Default)]
pub struct DischargeReport {
    pub obligations: Vec<ContractObligation>,
}

impl DischargeReport {
    pub fn proven_count(&self) -> usize {
        self.obligations
            .iter()
            .filter(|o| matches!(o.status, Status::Proven { .. }))
            .count()
    }

    pub fn runtime_count(&self) -> usize {
        self.obligations
            .iter()
            .filter(|o| matches!(o.status, Status::Runtime { .. }))
            .count()
    }

    pub fn unknown_count(&self) -> usize {
        self.obligations
            .iter()
            .filter(|o| matches!(o.status, Status::Unknown { .. }))
            .count()
    }

    /// Was a given obligation discharged statically? The lower pass
    /// consults this to decide whether to emit the runtime trap.
    pub fn is_proven(&self, function: &str, kind: ObligationKind, index: usize) -> bool {
        self.obligations.iter().any(|o| {
            o.function == function
                && o.kind == kind
                && o.index == index
                && matches!(o.status, Status::Proven { .. })
        })
    }

    /// Serialize to the canonical JSON shape consumed by CI / agent
    /// tooling. Mirrors the format documented in docs/contracts.md:
    /// per-obligation entries plus aggregate counts.
    pub fn to_json(&self) -> String {
        let mut s = String::new();
        s.push_str("{\n");
        s.push_str(&format!("  \"proven\": {},\n", self.proven_count()));
        s.push_str(&format!("  \"runtime\": {},\n", self.runtime_count()));
        s.push_str(&format!("  \"unknown\": {},\n", self.unknown_count()));
        s.push_str("  \"obligations\": [\n");
        for (i, o) in self.obligations.iter().enumerate() {
            let (status, tier, reason) = match &o.status {
                Status::Proven { tier } => (
                    "proven",
                    match tier {
                        Tier::Syntactic => "syntactic",
                        Tier::Smt => "smt",
                    },
                    String::new(),
                ),
                Status::Runtime { reason } => ("runtime", "", json_escape(reason)),
                Status::Unknown { reason } => ("unknown", "", json_escape(reason)),
            };
            s.push_str("    {");
            s.push_str(&format!("\"function\": \"{}\"", json_escape(&o.function)));
            s.push_str(&format!(
                ", \"clause\": \"{}\"",
                match o.kind {
                    ObligationKind::Requires => "requires",
                    ObligationKind::Ensures => "ensures",
                    ObligationKind::CallSite => "call_site",
                }
            ));
            s.push_str(&format!(", \"index\": {}", o.index));
            s.push_str(&format!(", \"status\": \"{}\"", status));
            if !tier.is_empty() {
                s.push_str(&format!(", \"tier\": \"{}\"", tier));
            }
            if !reason.is_empty() {
                s.push_str(&format!(", \"reason\": \"{}\"", reason));
            }
            s.push('}');
            if i + 1 < self.obligations.len() {
                s.push(',');
            }
            s.push('\n');
        }
        s.push_str("  ]\n");
        s.push_str("}\n");
        s
    }
}

fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

/// Run the discharge pass against an entire compilation unit and
/// return a fully-populated `DischargeReport`. `cfg.enable` controls
/// whether the SMT tier runs at all; tier-1 syntactic discharge runs
/// unconditionally because it's cheap and there's no failure mode
/// to surface.
pub fn discharge_file(file: &File, cfg: &DischargeConfig) -> DischargeReport {
    let smt = if cfg.enable {
        Some(smt::SmtBackend::detect())
    } else {
        None
    };

    let mut report = DischargeReport::default();
    for item in &file.items {
        if let Item::Fn(f) = item {
            collect_for_fn(f, &mut report, cfg, smt.as_ref(), &[]);
        }
        // TODO: also walk impl blocks once typed impls land — their
        // methods carry the same requires/ensures shape.
    }

    // I1: walk the file again to discharge per-call-site precondition
    // obligations. Adds new entries with kind=CallSite; the lower
    // pass doesn't act on them (the callee's @requires trap stays in
    // for defense in depth) but the report shows which call sites
    // are statically safe.
    callsite::run(file, cfg, &mut report);

    report
}

fn collect_for_fn(
    f: &FnDecl,
    report: &mut DischargeReport,
    cfg: &DischargeConfig,
    smt: Option<&SmtBackend>,
    module_path: &[String],
) {
    let fn_name = mangled_name(module_path, &f.name);
    let params = build_params(f);
    let unsigned_param_names = unsigned_param_names_for(f);

    let result_type = type_label_from(&f.return_type);

    for (i, expr) in f.requires.iter().enumerate() {
        let mut ob = ContractObligation {
            function: fn_name.clone(),
            kind: ObligationKind::Requires,
            index: i,
            expr: expr.clone(),
            params: params.clone(),
            result_type: None,
            assumptions: Vec::new(),
            body_result_expr: None,
            unsigned_param_names: unsigned_param_names.clone(),
            status: Status::Runtime {
                reason: "no tier matched".to_string(),
            },
        };
        run_pipeline(&mut ob, cfg, smt);
        report.obligations.push(ob);
    }

    // H1 — body-aware ensures discharge. The @ensures obligation
    // gets assumed @requires clauses and a model for `result`
    // derived from the function body when it's a single straight-
    // line `return EXPR;`. Anything more complex falls back to the
    // universal-tautology encoding (still discharges trichotomy /
    // De Morgan / etc).
    let assumptions = f.requires.clone();
    let body_result_expr = extract_straight_line_return(&f.body);

    for (i, expr) in f.ensures.iter().enumerate() {
        let mut ob = ContractObligation {
            function: fn_name.clone(),
            kind: ObligationKind::Ensures,
            index: i,
            expr: expr.clone(),
            params: params.clone(),
            result_type: Some(result_type.clone()),
            assumptions: assumptions.clone(),
            body_result_expr: body_result_expr.clone(),
            unsigned_param_names: unsigned_param_names.clone(),
            status: Status::Runtime {
                reason: "no tier matched".to_string(),
            },
        };
        run_pipeline(&mut ob, cfg, smt);
        report.obligations.push(ob);
    }
}

/// Names of the function's parameters whose AST type is unsigned
/// (`u8` / `u16` / `u32` / `u64` / `usize`). Used by the tier-1
/// syntactic discharger to recognize `@requires(n >= 0)` for an
/// unsigned `n` as always-true.
fn unsigned_param_names_for(f: &FnDecl) -> Vec<String> {
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

/// If the function body is a single `return EXPR;` statement (or a
/// block whose only statement is that return), return the expression.
/// Anything else — multiple statements, branches, loops, calls to
/// user functions, anything outside the supported subset — yields
/// `None`. The SMT tier still runs the universal-tautology encoding
/// when this is `None`.
fn extract_straight_line_return(body: &crate::ast::Block) -> Option<Expr> {
    if body.stmts.len() != 1 {
        return None;
    }
    let stmt = &body.stmts[0];
    let crate::ast::Stmt::Return { value: Some(e), .. } = stmt else {
        return None;
    };
    if !expr_in_supported_subset(e) {
        return None;
    }
    Some(e.clone())
}

fn run_pipeline(ob: &mut ContractObligation, cfg: &DischargeConfig, smt: Option<&SmtBackend>) {
    // Tier 1: always on.
    if syntactic::try_discharge(ob) {
        ob.status = Status::Proven {
            tier: Tier::Syntactic,
        };
        return;
    }
    // Tier 2: only when --prove and Z3 is available.
    if cfg.enable {
        match smt {
            Some(backend) => {
                match backend.try_discharge(ob, cfg.smt_budget_ms, cfg.cache_root.as_deref()) {
                    smt::SmtResult::Proven => {
                        ob.status = Status::Proven { tier: Tier::Smt };
                        return;
                    }
                    smt::SmtResult::Failed(reason) => {
                        ob.status = Status::Runtime {
                            reason: hint_for_counterexample(ob, &reason),
                        };
                        return;
                    }
                    smt::SmtResult::Timeout => {
                        ob.status = Status::Unknown {
                            reason: hint_for_timeout(ob, cfg.smt_budget_ms),
                        };
                        return;
                    }
                    smt::SmtResult::Unsupported(reason) => {
                        ob.status = Status::Runtime { reason };
                        return;
                    }
                }
            }
            None => {
                ob.status = Status::Runtime {
                    reason: "z3 not on PATH — install z3 to enable SMT tier".to_string(),
                };
                return;
            }
        }
    }
    // Tier 3 (runtime) is the default already set on the obligation.
}

/// H3: turn a generic "Z3 found a counterexample" message into a
/// kind-aware structured hint. The shape depends on what kind of
/// obligation we just tried to discharge:
///
///   - For `@requires`: the precondition isn't universally true,
///     and we don't yet have call-site context. Suggest the call-
///     site path, plus an alternative if the user can weaken the
///     precondition. (Body-aware path-sensitive precondition
///     discharge is a v2 item.)
///
///   - For `@ensures`: the postcondition doesn't follow from
///     `(requires ∧ body)`. Either the precondition is too weak
///     for what the body actually computes, or the postcondition
///     overpromises. Suggest both.
fn hint_for_counterexample(ob: &ContractObligation, base: &str) -> String {
    match ob.kind {
        ObligationKind::Requires => format!(
            "{} (in {}). v1 SMT proves obligations that are universally \
            true over their parameters; preconditions are typically only \
            discharged via call-site analysis (a v2 feature). The runtime \
            check stays in. Alternative: weaken the @requires clause to \
            something universally true, or split it into smaller clauses \
            that the tier-1 syntactic discharger can catch.",
            base, ob.function
        ),
        ObligationKind::Ensures => {
            let body_note = match &ob.body_result_expr {
                Some(_) => "the function body's return expression",
                None => "an unmodeled function body (loops / branches / calls)",
            };
            format!(
                "{} (in {}). Given the declared @requires and {}, Z3 found \
                inputs for which @ensures does not hold. Two ways to fix: \
                (a) strengthen @requires to exclude the counterexample, \
                or (b) weaken @ensures to match what the body actually \
                computes. The runtime check stays in until then.",
                base, ob.function, body_note
            )
        }
        ObligationKind::CallSite => format!(
            "{} (at {}). The caller's @requires and the substituted \
            arguments don't statically guarantee the callee's @requires. \
            Either strengthen the caller's @requires, or guard the call \
            with an explicit precondition check.",
            base, ob.function
        ),
    }
}

fn hint_for_timeout(ob: &ContractObligation, budget_ms: u64) -> String {
    let kind_label = match ob.kind {
        ObligationKind::Requires => "@requires",
        ObligationKind::Ensures => "@ensures",
        ObligationKind::CallSite => "call-site precondition",
    };
    format!(
        "SMT timed out after {} ms on {}[{}] in {}. Try splitting the \
        clause into smaller conjuncts (each is discharged independently), \
        or raise --prove-budget=<ms>. The runtime check is still emitted.",
        budget_ms, kind_label, ob.index, ob.function
    )
}

fn build_params(f: &FnDecl) -> Vec<ObligationParam> {
    let mut out = Vec::with_capacity(f.params.len());
    for p in &f.params {
        out.push(ObligationParam {
            name: p.name.clone(),
            type_label: type_label_from(&p.ty),
        });
    }
    out
}

/// Cheap structural label for a `TypeExpr`. The SMT encoder uses this
/// to map fastC types to Z3 sorts — anything not in the supported set
/// disqualifies an obligation from the SMT tier.
pub(crate) fn type_label_from(ty: &crate::ast::TypeExpr) -> String {
    use crate::ast::{PrimitiveType, TypeExpr};
    match ty {
        TypeExpr::Primitive(p) => match p {
            PrimitiveType::I8 | PrimitiveType::I16 | PrimitiveType::I32 => "i32".to_string(),
            PrimitiveType::I64 | PrimitiveType::Usize | PrimitiveType::Isize => "i64".to_string(),
            PrimitiveType::U8 | PrimitiveType::U16 | PrimitiveType::U32 => "i32".to_string(),
            PrimitiveType::U64 => "i64".to_string(),
            PrimitiveType::Bool => "bool".to_string(),
            PrimitiveType::F32 | PrimitiveType::F64 => "float".to_string(),
        },
        TypeExpr::Void => "void".to_string(),
        _ => "unknown".to_string(),
    }
}

fn mangled_name(module_path: &[String], name: &str) -> String {
    if module_path.is_empty() {
        name.to_string()
    } else {
        format!("{}__{}", module_path.join("__"), name)
    }
}

/// Helper used by both syntactic and SMT tiers — walks an `Expr`
/// recursively and returns true if every operator / leaf is in the
/// supported subset (linear integer arithmetic + boolean
/// combinators + comparisons). Anything else (function calls,
/// indexing, pointer ops, etc) disqualifies the obligation.
pub(crate) fn expr_in_supported_subset(e: &Expr) -> bool {
    match e {
        Expr::IntLit { .. } | Expr::BoolLit { .. } | Expr::Ident { .. } => true,
        Expr::Binary { op, lhs, rhs, .. } => {
            matches!(
                op,
                BinOp::Add
                    | BinOp::Sub
                    | BinOp::Mul
                    | BinOp::Eq
                    | BinOp::Ne
                    | BinOp::Lt
                    | BinOp::Le
                    | BinOp::Gt
                    | BinOp::Ge
                    | BinOp::And
                    | BinOp::Or
            ) && expr_in_supported_subset(lhs)
                && expr_in_supported_subset(rhs)
        }
        Expr::Unary { op, operand, .. } => {
            matches!(op, UnaryOp::Neg | UnaryOp::Not) && expr_in_supported_subset(operand)
        }
        Expr::Paren { inner, .. } => expr_in_supported_subset(inner),
        _ => false,
    }
}

/// Helper map used by SMT — maps each parameter name → its Z3 sort.
/// Plus `result` for `ensures`. Returns None if any type is
/// unsupported, signalling the SMT tier to skip this obligation.
pub(crate) fn build_sort_map(ob: &ContractObligation) -> Option<BTreeMap<String, &'static str>> {
    let mut out = BTreeMap::new();
    for p in &ob.params {
        let sort = sort_for(&p.type_label)?;
        out.insert(p.name.clone(), sort);
    }
    if matches!(ob.kind, ObligationKind::Ensures) {
        if let Some(rt) = ob.result_type.as_ref() {
            if rt != "void" {
                let sort = sort_for(rt)?;
                out.insert("result".to_string(), sort);
            }
        }
    }
    Some(out)
}

fn sort_for(label: &str) -> Option<&'static str> {
    match label {
        "i32" | "i64" => Some("Int"),
        "bool" => Some("Bool"),
        _ => None,
    }
}
