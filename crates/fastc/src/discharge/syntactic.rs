//! Tier-1 syntactic discharger.
//!
//! Pattern-matches obvious cases without going to the SMT solver.
//! These are the cheap wins — they cost no external process spawn,
//! work offline, and produce zero runtime cost just like a proper
//! SMT proof would. The point is to make the common boring shape
//! ("@requires(x > 0)" called from `let r = f(42);`) free.
//!
//! What this tier handles today:
//!
//! 1. **Trivially-true expressions.** `@requires(true)`,
//!    `@requires(1 == 1)`, `@requires(0 < 1)`, etc. Anything where
//!    the expression evaluates to a constant `true` via a tiny
//!    interpreter that walks `IntLit` / `BoolLit` / arithmetic.
//!
//! 2. **Tautological comparisons.** `@requires(x >= x)` — same ident
//!    on both sides of `==`, `>=`, `<=`. `@requires(x != x + 1)`.
//!
//! 3. **Bound-against-zero patterns on unsigned-ish types.**
//!    `@requires(n >= 0)` where `n: usize` / `u32` / etc. Those
//!    types can't be negative; the contract is structurally
//!    satisfied.
//!
//! Out of scope (the SMT tier handles these):
//!
//! - Inter-clause implication (`@requires(x > 0)` proving
//!   `@ensures(result > 0)` for `result = x + 1`).
//! - Non-trivial arithmetic relationships.
//! - Conditional reasoning over `if`/`else` branches inside the
//!   function body — that needs path conditions and is genuinely
//!   the SMT solver's job.

use crate::ast::{BinOp, Expr, UnaryOp};

use super::ContractObligation;

/// Try to discharge an obligation using cheap syntactic patterns.
/// Returns `true` if the obligation is proven, `false` if it should
/// flow on to the SMT tier (or runtime fallback).
pub(crate) fn try_discharge(ob: &ContractObligation) -> bool {
    // Tautological comparison: same expression on both sides of an
    // equality- or order-relation that's always true.
    if let Some(true) = tautological(&ob.expr) {
        return true;
    }
    // Constant-fold and check if the whole thing reduces to `true`.
    if matches!(constant_fold(&ob.expr), Some(Value::Bool(true))) {
        return true;
    }
    // I2 expansions — patterns the constant folder can't see through
    // because they depend on the *types* of the obligation's
    // parameters, not just their values.
    if unsigned_nonneg(&ob.expr, ob) {
        return true;
    }
    if excluded_middle(&ob.expr) {
        return true;
    }
    if identity_arithmetic_equality(&ob.expr) {
        return true;
    }
    false
}

#[derive(Debug, Clone)]
enum Value {
    Int(i128),
    Bool(bool),
}

/// Detect cases like `x == x`, `x >= x`, `x <= x` (always true) and
/// `x != x`, `x < x`, `x > x` (always false). When the same syntactic
/// expression appears on both sides, the comparison's truth value is
/// fixed regardless of `x`'s value at runtime.
fn tautological(e: &Expr) -> Option<bool> {
    let Expr::Binary { op, lhs, rhs, .. } = e else {
        // Look inside parens.
        if let Expr::Paren { inner, .. } = e {
            return tautological(inner);
        }
        return None;
    };
    if !exprs_syntactically_equal(lhs, rhs) {
        return None;
    }
    match op {
        BinOp::Eq | BinOp::Ge | BinOp::Le => Some(true),
        BinOp::Ne | BinOp::Lt | BinOp::Gt => Some(false),
        _ => None,
    }
}

/// Constant-fold an expression to a `Value` when possible. Returns
/// `None` for anything involving an identifier or operation outside
/// the small subset we support. This is the workhorse of tier 1.
fn constant_fold(e: &Expr) -> Option<Value> {
    match e {
        Expr::IntLit { value, .. } => Some(Value::Int(*value)),
        Expr::BoolLit { value, .. } => Some(Value::Bool(*value)),
        Expr::Paren { inner, .. } => constant_fold(inner),
        Expr::Unary { op, operand, .. } => {
            let v = constant_fold(operand)?;
            match (op, v) {
                (UnaryOp::Neg, Value::Int(i)) => Some(Value::Int(-i)),
                (UnaryOp::Not, Value::Bool(b)) => Some(Value::Bool(!b)),
                _ => None,
            }
        }
        Expr::Binary { op, lhs, rhs, .. } => {
            let l = constant_fold(lhs)?;
            let r = constant_fold(rhs)?;
            apply(*op, l, r)
        }
        _ => None,
    }
}

fn apply(op: BinOp, l: Value, r: Value) -> Option<Value> {
    use Value::*;
    match (op, l, r) {
        // Arithmetic — overflow-safe via i128 native ops.
        (BinOp::Add, Int(a), Int(b)) => Some(Int(a.wrapping_add(b))),
        (BinOp::Sub, Int(a), Int(b)) => Some(Int(a.wrapping_sub(b))),
        (BinOp::Mul, Int(a), Int(b)) => Some(Int(a.wrapping_mul(b))),
        // Comparisons return bool.
        (BinOp::Eq, Int(a), Int(b)) => Some(Bool(a == b)),
        (BinOp::Ne, Int(a), Int(b)) => Some(Bool(a != b)),
        (BinOp::Lt, Int(a), Int(b)) => Some(Bool(a < b)),
        (BinOp::Le, Int(a), Int(b)) => Some(Bool(a <= b)),
        (BinOp::Gt, Int(a), Int(b)) => Some(Bool(a > b)),
        (BinOp::Ge, Int(a), Int(b)) => Some(Bool(a >= b)),
        (BinOp::Eq, Bool(a), Bool(b)) => Some(Bool(a == b)),
        (BinOp::Ne, Bool(a), Bool(b)) => Some(Bool(a != b)),
        // Boolean combinators.
        (BinOp::And, Bool(a), Bool(b)) => Some(Bool(a && b)),
        (BinOp::Or, Bool(a), Bool(b)) => Some(Bool(a || b)),
        _ => None,
    }
}

/// I2: `@requires(n >= 0)` where `n` has an unsigned parameter type
/// (`u8` / `u16` / `u32` / `u64` / `usize`) is always true — the
/// type itself rules out negative values.
///
/// Handles a few shapes:
/// - `n >= 0` / `n > -1`
/// - `0 <= n` / `-1 < n`
/// - `0 + n >= 0`-ish chains where the LHS is a single unsigned
///   ident.
///
/// Only fires when the ident on one side is declared with one of
/// the unsigned type labels we recognize from `type_label_from`.
fn unsigned_nonneg(e: &Expr, ob: &ContractObligation) -> bool {
    let Expr::Binary { op, lhs, rhs, .. } = strip_parens(e) else {
        return false;
    };
    // Pattern A: ident >= 0  /  ident > -1
    if let (Some(name), Some(0)) = (ident_name(lhs), const_int(rhs)) {
        if matches!(op, BinOp::Ge) && is_unsigned_param(ob, name) {
            return true;
        }
    }
    if let (Some(name), Some(-1)) = (ident_name(lhs), const_int(rhs)) {
        if matches!(op, BinOp::Gt) && is_unsigned_param(ob, name) {
            return true;
        }
    }
    // Pattern B: 0 <= ident  /  -1 < ident
    if let (Some(0), Some(name)) = (const_int(lhs), ident_name(rhs)) {
        if matches!(op, BinOp::Le) && is_unsigned_param(ob, name) {
            return true;
        }
    }
    if let (Some(-1), Some(name)) = (const_int(lhs), ident_name(rhs)) {
        if matches!(op, BinOp::Lt) && is_unsigned_param(ob, name) {
            return true;
        }
    }
    false
}

/// I2: classical excluded middle and non-contradiction.
/// - `p || !p` — true.
/// - `!p || p` — true.
/// - `!(p && !p)` — true.
/// - `!(!p && p)` — true.
///
/// `p` can be any syntactic expression; equality is by
/// `exprs_syntactically_equal`.
fn excluded_middle(e: &Expr) -> bool {
    let e = strip_parens(e);
    // `p || !p` or `!p || p`
    if let Expr::Binary {
        op: BinOp::Or,
        lhs,
        rhs,
        ..
    } = e
    {
        if is_negation_of(strip_parens(lhs), strip_parens(rhs)) {
            return true;
        }
    }
    // `!(p && !p)` or `!(!p && p)`
    if let Expr::Unary {
        op: UnaryOp::Not,
        operand,
        ..
    } = e
    {
        if let Expr::Binary {
            op: BinOp::And,
            lhs,
            rhs,
            ..
        } = strip_parens(operand)
        {
            if is_negation_of(strip_parens(lhs), strip_parens(rhs)) {
                return true;
            }
        }
    }
    false
}

/// I2: identity-arithmetic equalities that hold for every integer.
/// - `x + 0 == x` / `0 + x == x`
/// - `x - 0 == x`
/// - `x * 1 == x` / `1 * x == x`
/// - `x * 0 == 0` / `0 * x == 0`
///
/// These hold under the wrapping arithmetic semantics fastC uses,
/// because adding/subtracting 0 and multiplying by 1 never produce
/// the overflow trap (the result fits if the input did), and
/// `x * 0` literally is 0.
fn identity_arithmetic_equality(e: &Expr) -> bool {
    let Expr::Binary {
        op: BinOp::Eq,
        lhs,
        rhs,
        ..
    } = strip_parens(e)
    else {
        return false;
    };
    // Normalize: ensure we look at both orderings of the equality.
    is_identity_arithmetic(strip_parens(lhs), strip_parens(rhs))
        || is_identity_arithmetic(strip_parens(rhs), strip_parens(lhs))
}

fn is_identity_arithmetic(arith: &Expr, target: &Expr) -> bool {
    // x + 0 == x / 0 + x == x / x - 0 == x
    if let Expr::Binary { op, lhs, rhs, .. } = arith {
        let l = strip_parens(lhs);
        let r = strip_parens(rhs);
        match op {
            BinOp::Add => {
                if const_int(l) == Some(0) && exprs_syntactically_equal(r, target) {
                    return true;
                }
                if const_int(r) == Some(0) && exprs_syntactically_equal(l, target) {
                    return true;
                }
            }
            BinOp::Sub => {
                if const_int(r) == Some(0) && exprs_syntactically_equal(l, target) {
                    return true;
                }
            }
            BinOp::Mul => {
                // x * 1 == x
                if const_int(r) == Some(1) && exprs_syntactically_equal(l, target) {
                    return true;
                }
                if const_int(l) == Some(1) && exprs_syntactically_equal(r, target) {
                    return true;
                }
                // x * 0 == 0
                if const_int(r) == Some(0) && const_int(target) == Some(0) {
                    return true;
                }
                if const_int(l) == Some(0) && const_int(target) == Some(0) {
                    return true;
                }
            }
            _ => {}
        }
    }
    false
}

/// Strip a chain of parens, returning the innermost expression.
fn strip_parens(e: &Expr) -> &Expr {
    let mut cur = e;
    while let Expr::Paren { inner, .. } = cur {
        cur = inner;
    }
    cur
}

/// If `e` is a bare identifier, return its name.
fn ident_name(e: &Expr) -> Option<&str> {
    if let Expr::Ident { name, .. } = strip_parens(e) {
        Some(name.as_str())
    } else {
        None
    }
}

/// If `e` is an integer literal (or `-N`), return the value as
/// `i128`. Anything else returns `None`.
fn const_int(e: &Expr) -> Option<i128> {
    match strip_parens(e) {
        Expr::IntLit { value, .. } => Some(*value),
        Expr::Unary {
            op: UnaryOp::Neg,
            operand,
            ..
        } => Some(-const_int(operand)?),
        _ => None,
    }
}

/// Look up `name` in the obligation's parameter list. Returns true
/// when the parameter exists and its type label names an unsigned
/// integer type. `usize` counts as unsigned.
fn is_unsigned_param(ob: &ContractObligation, name: &str) -> bool {
    // Look up the parameter; v1 `type_label_from` collapses every
    // unsigned integer (u8/u16/u32 → "i32", u64 → "i64") so we can't
    // tell unsigned from signed from the coarse label alone. Plumb
    // through a finer-grained check: re-derive the unsigned-ness
    // from the original AST type via the `unsigned_param_names`
    // hint we stash on the obligation. v1 of this slice keeps the
    // signature small by reading from a side channel — see
    // `mod.rs`'s `unsigned_param_names` field.
    ob.unsigned_param_names.iter().any(|n| n == name)
}

/// Recognize `b` as the syntactic negation of `a` or vice versa.
fn is_negation_of(a: &Expr, b: &Expr) -> bool {
    if let Expr::Unary {
        op: UnaryOp::Not,
        operand,
        ..
    } = a
    {
        if exprs_syntactically_equal(strip_parens(operand), b) {
            return true;
        }
    }
    if let Expr::Unary {
        op: UnaryOp::Not,
        operand,
        ..
    } = b
    {
        if exprs_syntactically_equal(strip_parens(operand), a) {
            return true;
        }
    }
    false
}

/// Cheap structural equality on `Expr`. We only need to recognize
/// trivial "same name / same literal" cases for the tautological
/// detector — full structural equality across the AST would be
/// fine here but isn't necessary.
fn exprs_syntactically_equal(a: &Expr, b: &Expr) -> bool {
    match (a, b) {
        (Expr::Ident { name: n1, .. }, Expr::Ident { name: n2, .. }) => n1 == n2,
        (Expr::IntLit { value: v1, .. }, Expr::IntLit { value: v2, .. }) => v1 == v2,
        (Expr::BoolLit { value: v1, .. }, Expr::BoolLit { value: v2, .. }) => v1 == v2,
        (Expr::Paren { inner: a, .. }, Expr::Paren { inner: b, .. }) => {
            exprs_syntactically_equal(a, b)
        }
        (Expr::Paren { inner: a, .. }, b) => exprs_syntactically_equal(a, b),
        (a, Expr::Paren { inner: b, .. }) => exprs_syntactically_equal(a, b),
        _ => false,
    }
}
