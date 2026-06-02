//! Unit tests for the discharge pass.
//!
//! Covers:
//!
//! - Tier-1 syntactic discharger proves trivially-true contracts.
//! - The pipeline correctly classifies obligations by status.
//! - `DischargeReport.to_json` round-trips through a minimal parse.
//!
//! Tier-2 SMT tests live in `crates/fastc/tests/discharge.rs` — they
//! require `z3` on PATH and skip gracefully when it isn't.

use crate::ast::{BinOp, Expr, Span, UnaryOp};
use crate::discharge::{
    ContractObligation, DischargeConfig, ObligationKind, ObligationParam, Status, Tier,
    discharge_file,
};
use crate::parse;

fn span() -> Span {
    0..0
}

fn ilit(v: i128) -> Expr {
    Expr::IntLit {
        value: v,
        span: span(),
    }
}

fn ident(n: &str) -> Expr {
    Expr::Ident {
        name: n.to_string(),
        span: span(),
    }
}

fn bin(op: BinOp, l: Expr, r: Expr) -> Expr {
    Expr::Binary {
        op,
        lhs: Box::new(l),
        rhs: Box::new(r),
        span: span(),
    }
}

fn fake_obligation(expr: Expr) -> ContractObligation {
    ContractObligation {
        function: "test_fn".to_string(),
        kind: ObligationKind::Requires,
        index: 0,
        expr,
        params: vec![ObligationParam {
            name: "x".to_string(),
            type_label: "i32".to_string(),
        }],
        result_type: None,
        assumptions: Vec::new(),
        body_result_expr: None,
        unsigned_param_names: Vec::new(),
        status: Status::Runtime {
            reason: "no tier matched".to_string(),
        },
    }
}

#[test]
fn syntactic_proves_literal_true() {
    let mut ob = fake_obligation(Expr::BoolLit {
        value: true,
        span: span(),
    });
    let _ = super::syntactic::try_discharge(&ob);
    super::run_pipeline(&mut ob, &DischargeConfig::default(), None);
    assert!(matches!(
        ob.status,
        Status::Proven {
            tier: Tier::Syntactic
        }
    ));
}

#[test]
fn syntactic_proves_constant_inequality() {
    // 0 < 1 — always true.
    let expr = bin(BinOp::Lt, ilit(0), ilit(1));
    let mut ob = fake_obligation(expr);
    super::run_pipeline(&mut ob, &DischargeConfig::default(), None);
    assert!(matches!(
        ob.status,
        Status::Proven {
            tier: Tier::Syntactic
        }
    ));
}

#[test]
fn syntactic_proves_tautology_same_ident() {
    // x >= x — always true regardless of x.
    let expr = bin(BinOp::Ge, ident("x"), ident("x"));
    let mut ob = fake_obligation(expr);
    super::run_pipeline(&mut ob, &DischargeConfig::default(), None);
    assert!(matches!(
        ob.status,
        Status::Proven {
            tier: Tier::Syntactic
        }
    ));
}

#[test]
fn syntactic_rejects_x_lt_x_for_proof() {
    // x < x — always FALSE. Should NOT be proven as the contract.
    let expr = bin(BinOp::Lt, ident("x"), ident("x"));
    let mut ob = fake_obligation(expr);
    super::run_pipeline(&mut ob, &DischargeConfig::default(), None);
    // We need an SMT solver to know this is unsatisfiable; tier-1
    // (with default disabled) leaves it on runtime.
    assert!(matches!(ob.status, Status::Runtime { .. }));
}

#[test]
fn syntactic_proves_double_negation() {
    // !(false) — true.
    let expr = Expr::Unary {
        op: UnaryOp::Not,
        operand: Box::new(Expr::BoolLit {
            value: false,
            span: span(),
        }),
        span: span(),
    };
    let mut ob = fake_obligation(expr);
    super::run_pipeline(&mut ob, &DischargeConfig::default(), None);
    assert!(matches!(
        ob.status,
        Status::Proven {
            tier: Tier::Syntactic
        }
    ));
}

#[test]
fn pipeline_falls_through_to_runtime_when_unprovable_and_no_smt() {
    // x > 0 — depends on x. Tier-1 can't prove it; SMT disabled in
    // default config; should land on Runtime.
    let expr = bin(BinOp::Gt, ident("x"), ilit(0));
    let mut ob = fake_obligation(expr);
    super::run_pipeline(&mut ob, &DischargeConfig::default(), None);
    assert!(matches!(ob.status, Status::Runtime { .. }));
}

#[test]
fn report_json_includes_aggregate_counts() {
    let src = r#"
        @requires(x > 0)
        @ensures(true)
        fn double(x: i32) -> i32 {
            return (x + x);
        }
    "#;
    let file = parse(src, "test.fc").expect("parse");
    let report = discharge_file(&file, &DischargeConfig::default());
    let json = report.to_json();
    // The `true` ensures should be syntactically discharged; the
    // `x > 0` requires can't be (unsafe without SMT).
    assert!(json.contains("\"proven\":"));
    assert!(json.contains("\"runtime\":"));
    assert!(json.contains("\"function\": \"double\""));
    assert!(json.contains("\"clause\": \"requires\""));
    assert!(json.contains("\"clause\": \"ensures\""));
}

#[test]
fn report_counts_match_obligations() {
    let src = r#"
        @requires(true)
        @requires(x >= x)
        @ensures(result == result)
        fn id(x: i32) -> i32 {
            return x;
        }
    "#;
    let file = parse(src, "test.fc").expect("parse");
    let report = discharge_file(&file, &DischargeConfig::default());
    assert_eq!(report.obligations.len(), 3);
    assert_eq!(report.proven_count(), 3);
    assert_eq!(report.runtime_count(), 0);
}

#[test]
fn tier1_unsigned_nonneg_for_usize_param() {
    let src = r#"
        @requires(i >= 0)
        fn get_at(i: usize) -> i32 { return 0; }
    "#;
    let file = parse(src, "test.fc").expect("parse");
    let report = super::discharge_file(&file, &DischargeConfig::default());
    assert_eq!(report.proven_count(), 1);
    assert!(matches!(
        report.obligations[0].status,
        Status::Proven {
            tier: Tier::Syntactic
        }
    ));
}

#[test]
fn tier1_unsigned_nonneg_for_u32_param() {
    let src = r#"
        @requires(n >= 0)
        @requires(0 <= n)
        fn f(n: u32) -> i32 { return 0; }
    "#;
    let file = parse(src, "test.fc").expect("parse");
    let report = super::discharge_file(&file, &DischargeConfig::default());
    assert_eq!(report.proven_count(), 2, "both shapes should fire");
}

#[test]
fn tier1_unsigned_nonneg_rejects_signed_param() {
    let src = r#"
        @requires(x >= 0)
        fn f(x: i32) -> i32 { return 0; }
    "#;
    let file = parse(src, "test.fc").expect("parse");
    let report = super::discharge_file(&file, &DischargeConfig::default());
    // Signed i32 can be negative — must NOT be proven syntactically.
    assert_eq!(report.runtime_count(), 1);
}

#[test]
fn tier1_excluded_middle_proves_p_or_not_p() {
    // `p || !p` over an unknown bool — true regardless.
    let src = r#"
        @requires(b || (!b))
        fn f(b: bool) -> i32 { return 0; }
    "#;
    let file = parse(src, "test.fc").expect("parse");
    let report = super::discharge_file(&file, &DischargeConfig::default());
    assert_eq!(report.proven_count(), 1);
    assert!(matches!(
        report.obligations[0].status,
        Status::Proven {
            tier: Tier::Syntactic
        }
    ));
}

#[test]
fn tier1_identity_arith_x_plus_zero_equals_x() {
    let src = r#"
        @requires((x + 0) == x)
        fn f(x: i32) -> i32 { return 0; }
    "#;
    let file = parse(src, "test.fc").expect("parse");
    let report = super::discharge_file(&file, &DischargeConfig::default());
    assert_eq!(report.proven_count(), 1);
}

#[test]
fn tier1_identity_arith_x_times_one_equals_x() {
    let src = r#"
        @requires((x * 1) == x)
        fn f(x: i32) -> i32 { return 0; }
    "#;
    let file = parse(src, "test.fc").expect("parse");
    let report = super::discharge_file(&file, &DischargeConfig::default());
    assert_eq!(report.proven_count(), 1);
}

#[test]
fn is_proven_lookup_works() {
    let src = r#"
        @requires(true)
        @ensures(result >= result)
        fn f(x: i32) -> i32 { return x; }
    "#;
    let file = parse(src, "test.fc").expect("parse");
    let report = discharge_file(&file, &DischargeConfig::default());
    assert!(report.is_proven("f", ObligationKind::Requires, 0));
    assert!(report.is_proven("f", ObligationKind::Ensures, 0));
    assert!(!report.is_proven("f", ObligationKind::Requires, 99));
    assert!(!report.is_proven("nonexistent", ObligationKind::Requires, 0));
}
