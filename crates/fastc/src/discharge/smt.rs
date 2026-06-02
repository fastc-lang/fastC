//! Tier-2 SMT backend.
//!
//! Shells out to `z3 -smt2 -in` with generated SMT-LIB. Per-obligation
//! budget is enforced both via `(set-option :timeout <ms>)` (the
//! solver-side bound) and a process-level kill at 2× the budget (so a
//! pathological Z3 run can't wedge the build forever).
//!
//! The encoding:
//!
//! 1. Every function parameter becomes a Z3 constant of the matching
//!    sort (Int for i32/i64/usize/etc, Bool for bool).
//! 2. For an `@ensures` obligation, an additional `result` constant
//!    of the function's return type is declared.
//! 3. The obligation expression is encoded recursively (`+`, `-`,
//!    `*`, comparisons, `&&`, `||`, `!`) to SMT-LIB.
//! 4. We `assert (not <obligation>)` and check for `unsat` — the
//!    classic "no counterexample" formulation. `unsat` ⇒ proven.
//!    `sat` ⇒ Z3 found a counterexample; the obligation might still
//!    be true under richer reasoning but the runtime check stays.
//!
//! ## Why shell out instead of using the `z3` crate
//!
//! - Z3 stays optional. Users without Z3 installed still get the
//!   tier-1 discharger and the runtime fallback — the workflow
//!   degrades cleanly instead of "you can't build fastC code".
//! - No build-time linking against libz3. fastC's release binary
//!   stays small (53 KB hello stays 53 KB).
//! - We can cache by SMT-LIB text, which is exactly what the user
//!   would see if they ran z3 manually for debugging.

use std::io::Write;
use std::process::{Command, Stdio};

use crate::ast::{BinOp, Expr, UnaryOp};

use super::{ContractObligation, ObligationKind, build_sort_map, expr_in_supported_subset};
use std::path::Path;

/// Z3 backend, lazily detected at discharge-pass startup.
#[derive(Debug, Clone)]
pub struct SmtBackend {
    /// Absolute path to the `z3` binary. `None` means Z3 isn't on
    /// PATH; the SMT tier skips every obligation in that case.
    z3_path: Option<std::path::PathBuf>,
}

impl SmtBackend {
    /// Probe PATH for `z3` once. Cached for the lifetime of the
    /// discharge pass.
    pub fn detect() -> Self {
        let z3_path = std::env::var_os("PATH").and_then(|paths| {
            std::env::split_paths(&paths)
                .map(|d| d.join("z3"))
                .find(|p| p.is_file())
        });
        Self { z3_path }
    }

    pub fn is_available(&self) -> bool {
        self.z3_path.is_some()
    }

    /// Try to discharge one obligation. Returns `Proven` on `unsat`,
    /// `Failed` on `sat` (Z3 found a counterexample), `Timeout` when
    /// the budget elapses, and `Unsupported` when the obligation
    /// uses constructs the encoder doesn't support yet.
    ///
    /// H2: when `cache_root` is set, the SMT-LIB text we'd hand z3
    /// is hashed and a previous result reused if present. Cache
    /// hits skip the z3 spawn entirely.
    pub fn try_discharge(
        &self,
        ob: &ContractObligation,
        budget_ms: u64,
        cache_root: Option<&Path>,
    ) -> SmtResult {
        let Some(z3) = &self.z3_path else {
            return SmtResult::Unsupported("z3 not on PATH (tier-2 disabled)".to_string());
        };

        if !expr_in_supported_subset(&ob.expr) {
            return SmtResult::Unsupported(
                "expression uses operators outside the SMT-supported subset \
                 (function calls / pointer ops / nonlinear arithmetic etc)"
                    .to_string(),
            );
        }

        let Some(sorts) = build_sort_map(ob) else {
            return SmtResult::Unsupported(
                "parameter type doesn't map to a supported SMT sort".to_string(),
            );
        };

        let smt_lib = encode_obligation(ob, budget_ms, &sorts);

        // H2 cache lookup — exit immediately on hit.
        if let Some(root) = cache_root {
            if let Some(cached) = super::cache::lookup(root, &smt_lib) {
                return cached;
            }
        }

        // Run z3 with hard process-level timeout at 2x budget so a
        // wedged solver doesn't dominate the build.
        let hard_timeout = std::time::Duration::from_millis(budget_ms.saturating_mul(2).max(100));
        let mut child = match Command::new(z3)
            .args(["-smt2", "-in"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
        {
            Ok(c) => c,
            Err(e) => return SmtResult::Unsupported(format!("failed to invoke z3: {}", e)),
        };

        // Write the SMT-LIB script and drop stdin so Z3 sees EOF and
        // actually runs `(check-sat)` rather than waiting indefinitely
        // for more input. The `take` here releases the borrow before
        // we later wait for the child.
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(smt_lib.as_bytes());
            drop(stdin);
        }

        let start = std::time::Instant::now();
        let output = loop {
            match child.try_wait() {
                Ok(Some(_)) => break child.wait_with_output(),
                Ok(None) => {
                    if start.elapsed() >= hard_timeout {
                        let _ = child.kill();
                        return SmtResult::Timeout;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
                Err(e) => return SmtResult::Unsupported(format!("z3 wait error: {}", e)),
            }
        };

        let output = match output {
            Ok(o) => o,
            Err(e) => return SmtResult::Unsupported(format!("z3 output error: {}", e)),
        };

        let stdout = String::from_utf8_lossy(&output.stdout);
        let first = stdout.lines().next().unwrap_or("").trim();
        let result = match first {
            "unsat" => SmtResult::Proven,
            "sat" => {
                SmtResult::Failed("Z3 found a counterexample — runtime check retained".to_string())
            }
            "unknown" => SmtResult::Timeout,
            other => SmtResult::Unsupported(format!("unexpected z3 output: {}", other)),
        };

        // H2: persist the result for next time.
        if let Some(root) = cache_root {
            super::cache::store(root, &smt_lib, &result);
        }

        result
    }
}

#[derive(Debug)]
pub enum SmtResult {
    Proven,
    Failed(String),
    Timeout,
    Unsupported(String),
}

/// Produce the SMT-LIB script that asserts `not <obligation>` and
/// asks Z3 for a counterexample. `unsat` means no counterexample
/// exists ⇒ the obligation is universally true.
///
/// H1 body-aware encoding:
///   - Every `@requires` clause of the same function (passed in via
///     `ob.assumptions`) is asserted unconditionally before the
///     negated obligation. This turns "is this clause universally
///     true?" into "is this clause true given the preconditions?".
///   - If the function body is a single straight-line `return EXPR`
///     (extracted by `extract_straight_line_return` in mod.rs), we
///     add `(assert (= result EXPR))` so the obligation can
///     reference body-computed values via `result`.
///   - Anything more complex falls back to the prior universal-
///     tautology shape — still discharges trichotomy / De Morgan.
fn encode_obligation(
    ob: &ContractObligation,
    budget_ms: u64,
    sorts: &std::collections::BTreeMap<String, &'static str>,
) -> String {
    let mut out = String::new();
    out.push_str(&format!("(set-option :timeout {})\n", budget_ms));
    out.push_str("(set-logic QF_LIA)\n"); // quantifier-free linear int arith
    for (name, sort) in sorts {
        out.push_str(&format!("(declare-const {} {})\n", smt_name(name), sort));
    }

    // H1: assert every @requires clause (the assumption set).
    for a in &ob.assumptions {
        if !expr_in_supported_subset(a) {
            // Skip unsupported assumptions silently. The encoding
            // is still sound — fewer assumptions only means a
            // weaker proof, not an unsound one.
            continue;
        }
        out.push_str(&format!("(assert {})\n", encode_expr(a)));
    }

    // H1: model `result` from a straight-line body.
    if let Some(body_expr) = ob.body_result_expr.as_ref() {
        if sorts.contains_key("result") && expr_in_supported_subset(body_expr) {
            out.push_str(&format!("(assert (= result {}))\n", encode_expr(body_expr)));
        }
    }

    let body = encode_expr(&ob.expr);
    out.push_str(&format!("(assert (not {}))\n", body));
    out.push_str("(check-sat)\n");
    out
}

fn encode_expr(e: &Expr) -> String {
    match e {
        Expr::IntLit { value, .. } => {
            if *value < 0 {
                format!("(- {})", -value)
            } else {
                value.to_string()
            }
        }
        Expr::BoolLit { value, .. } => {
            if *value {
                "true".to_string()
            } else {
                "false".to_string()
            }
        }
        Expr::Ident { name, .. } => smt_name(name),
        Expr::Paren { inner, .. } => encode_expr(inner),
        Expr::Unary { op, operand, .. } => {
            let inner = encode_expr(operand);
            match op {
                UnaryOp::Neg => format!("(- {})", inner),
                UnaryOp::Not => format!("(not {})", inner),
                UnaryOp::BitNot => format!("(bvnot {})", inner),
            }
        }
        Expr::Binary { op, lhs, rhs, .. } => {
            let l = encode_expr(lhs);
            let r = encode_expr(rhs);
            let opname = match op {
                BinOp::Add => "+",
                BinOp::Sub => "-",
                BinOp::Mul => "*",
                BinOp::Eq => "=",
                BinOp::Ne => "distinct",
                BinOp::Lt => "<",
                BinOp::Le => "<=",
                BinOp::Gt => ">",
                BinOp::Ge => ">=",
                BinOp::And => "and",
                BinOp::Or => "or",
                _ => return format!("(error unsupported {:?})", op),
            };
            format!("({} {} {})", opname, l, r)
        }
        _ => "(error unsupported_expr)".to_string(),
    }
}

/// SMT-LIB identifiers can be most things, but we sanitize fastC
/// names to a conservative subset to avoid quoting headaches.
fn smt_name(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if c.is_ascii_alphanumeric() || c == '_' {
            out.push(c);
        } else {
            out.push('_');
        }
    }
    out
}

#[allow(dead_code)]
pub(super) fn obligation_smt_for_test(ob: &ContractObligation, budget_ms: u64) -> Option<String> {
    if !expr_in_supported_subset(&ob.expr) {
        return None;
    }
    let sorts = build_sort_map(ob)?;
    Some(encode_obligation(ob, budget_ms, &sorts))
}

// Helper exposed for testing the encoder without invoking Z3.
#[allow(dead_code)]
pub(super) fn _kind_helper(_k: ObligationKind) {}
