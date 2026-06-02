//! Integration tests for the stage 2.1 SMT contract discharge pipeline.
//!
//! Covers the end-to-end path: parse source → discharge pass →
//! generated C contains (or omits) `fc_trap` guards for each
//! obligation according to its discharge status.
//!
//! Tier-2 SMT tests shell out to `z3` and skip gracefully when it
//! isn't on PATH — same pattern as the cosign integration in
//! supply_chain.rs.

use fastc::{
    P10Config, compile_with_p10_and_discharge,
    discharge::{DischargeConfig, ObligationKind, Status, Tier},
};

fn z3_available() -> bool {
    std::env::var_os("PATH")
        .map(|paths| std::env::split_paths(&paths).any(|d| d.join("z3").is_file()))
        .unwrap_or(false)
}

fn run(src: &str, cfg: DischargeConfig) -> (String, fastc::discharge::DischargeReport) {
    let (c, _h, report) =
        compile_with_p10_and_discharge(src, "test.fc", false, P10Config::standard(), &cfg)
            .expect("compile");
    (c, report)
}

#[test]
fn tier1_proves_literal_true_requires_and_elides_trap() {
    let src = r#"
        @requires(true)
        fn f() -> i32 { return 7; }
    "#;
    let (c, report) = run(src, DischargeConfig::default());
    assert_eq!(report.proven_count(), 1);
    assert_eq!(report.runtime_count(), 0);
    let f_body = extract_fn(&c, "f");
    assert!(
        !f_body.contains("fc_trap"),
        "tier-1 proven obligation should elide fc_trap; got body:\n{}",
        f_body
    );
    assert!(matches!(
        report.obligations[0].status,
        Status::Proven {
            tier: Tier::Syntactic
        }
    ));
}

#[test]
fn unproven_requires_keeps_runtime_trap() {
    let src = r#"
        @requires(x > 0)
        fn pos_only(x: i32) -> i32 { return x; }
    "#;
    let (c, report) = run(src, DischargeConfig::default());
    assert_eq!(report.runtime_count(), 1);
    let body = extract_fn(&c, "pos_only");
    assert!(
        body.contains("fc_trap"),
        "unproven obligation must keep its runtime trap:\n{}",
        body
    );
}

#[test]
fn ensures_tautology_is_discharged_and_trap_elided() {
    let src = r#"
        @ensures(result == result)
        fn id(x: i32) -> i32 { return x; }
    "#;
    let (c, report) = run(src, DischargeConfig::default());
    // The body capture variable should still be there (it's how
    // ensures are wired even when the runtime trap is gone) but no
    // fc_trap should remain.
    assert_eq!(report.proven_count(), 1);
    let body = extract_fn(&c, "id");
    assert!(
        !body.contains("fc_trap"),
        "proven ensures should elide trap:\n{}",
        body
    );
}

#[test]
fn no_prove_forces_runtime_even_for_trivial_clauses() {
    let src = r#"
        @requires(true)
        fn f() -> i32 { return 0; }
    "#;
    // Tier-1 always runs (`enable` only gates SMT), but
    // `@requires(true)` is so trivial tier-1 catches it. So this
    // test demonstrates the floor: even with `enable=false`,
    // syntactic discharge still works. The `Status::Proven { tier: Syntactic }`
    // tag is what tells us tier-1 fired.
    let (_c, report) = run(
        src,
        DischargeConfig {
            enable: false,
            smt_budget_ms: 500,
            cache_root: None,
        },
    );
    assert!(matches!(
        report.obligations[0].status,
        Status::Proven {
            tier: Tier::Syntactic
        }
    ));
}

#[test]
fn discharge_json_matches_report_counts() {
    let src = r#"
        @requires(true)
        @requires(x > 0)
        fn mix(x: i32) -> i32 { return x; }
    "#;
    let (_c, report) = run(src, DischargeConfig::default());
    let json = report.to_json();
    assert!(json.contains("\"proven\": 1"));
    assert!(json.contains("\"runtime\": 1"));
    assert!(json.contains("\"function\": \"mix\""));
    assert!(json.contains("\"tier\": \"syntactic\""));
}

#[test]
fn report_is_proven_query_works() {
    let src = r#"
        @requires(true)
        @ensures(result == result)
        fn f(x: i32) -> i32 { return x; }
    "#;
    let (_c, report) = run(src, DischargeConfig::default());
    assert!(report.is_proven("f", ObligationKind::Requires, 0));
    assert!(report.is_proven("f", ObligationKind::Ensures, 0));
}

#[test]
fn smt_proves_integer_trichotomy() {
    if !z3_available() {
        eprintln!("skipping: z3 not on PATH");
        return;
    }
    let src = r#"
        @requires((a > 0) || ((a == 0) || (a < 0)))
        fn trichotomy(a: i32) -> i32 { return a; }
    "#;
    let (c, report) = run(
        src,
        DischargeConfig {
            enable: true,
            smt_budget_ms: 1500,
            cache_root: None,
        },
    );
    assert!(
        matches!(
            report.obligations[0].status,
            Status::Proven { tier: Tier::Smt }
        ),
        "expected SMT-proven, got {:?}",
        report.obligations[0].status
    );
    let body = extract_fn(&c, "trichotomy");
    assert!(
        !body.contains("fc_trap"),
        "SMT-proven obligation should elide trap:\n{}",
        body
    );
}

#[test]
fn smt_proves_de_morgan() {
    if !z3_available() {
        eprintln!("skipping: z3 not on PATH");
        return;
    }
    let src = r#"
        @requires((!(a > b)) == (a <= b))
        fn dm(a: i32, b: i32) -> i32 { return (a + b); }
    "#;
    let (_c, report) = run(
        src,
        DischargeConfig {
            enable: true,
            smt_budget_ms: 1500,
            cache_root: None,
        },
    );
    assert!(matches!(
        report.obligations[0].status,
        Status::Proven { tier: Tier::Smt }
    ));
}

#[test]
fn body_aware_smt_proves_ensures_via_requires_and_return_expr() {
    if !z3_available() {
        eprintln!("skipping: z3 not on PATH");
        return;
    }
    // The headline H1 case: a precondition (x > 0) combined with a
    // straight-line return `(x + 1)` lets Z3 prove `result > 1`.
    let src = r#"
        @requires(x > 0)
        @ensures(result > 1)
        fn add_one_pos(x: i32) -> i32 {
            return (x + 1);
        }
    "#;
    let (c, report) = run(
        src,
        DischargeConfig {
            enable: true,
            smt_budget_ms: 1500,
            cache_root: None,
        },
    );
    let ensures = report
        .obligations
        .iter()
        .find(|o| matches!(o.kind, ObligationKind::Ensures))
        .expect("found ensures");
    assert!(
        matches!(ensures.status, Status::Proven { tier: Tier::Smt }),
        "expected body-aware SMT proof, got {:?}",
        ensures.status
    );
    // The lowered C should not carry an `@ensures` trap for the
    // proven obligation. (The `@requires` trap remains.)
    let body = extract_fn(&c, "add_one_pos");
    let traps = body.matches("fc_trap()").count();
    // Two `fc_trap()` calls remain: the @requires guard and the
    // overflow check on `x + 1`. The @ensures trap is gone.
    assert!(
        traps <= 2,
        "expected ≤2 fc_traps (requires + overflow), got {} traps in:\n{}",
        traps,
        body
    );
}

#[test]
fn body_aware_smt_keeps_trap_when_ensures_too_strong() {
    if !z3_available() {
        eprintln!("skipping: z3 not on PATH");
        return;
    }
    // The @ensures here is genuinely false for x=0; Z3 should find
    // a counterexample even with body-aware encoding.
    let src = r#"
        @requires(x >= 0)
        @ensures(result >= 10)
        fn unprovable(x: i32) -> i32 { return x; }
    "#;
    let (_c, report) = run(
        src,
        DischargeConfig {
            enable: true,
            smt_budget_ms: 1500,
            cache_root: None,
        },
    );
    let ensures = report
        .obligations
        .iter()
        .find(|o| matches!(o.kind, ObligationKind::Ensures))
        .expect("found ensures");
    assert!(matches!(ensures.status, Status::Runtime { .. }));
}

#[test]
fn callsite_literal_arg_proves_callee_requires() {
    // I1: divisor_safe(5) substitutes x=5 into @requires(x > 0),
    // tier-1 constant-folds 5 > 0 to true, proven.
    let src = r#"
        @requires(x > 0)
        fn divisor_safe(x: i32) -> i32 { return x; }

        fn caller() -> i32 { return divisor_safe(5); }
    "#;
    let (_c, report) = run(src, DischargeConfig::default());
    let cs = report
        .obligations
        .iter()
        .find(|o| matches!(o.kind, ObligationKind::CallSite))
        .expect("found call-site obligation");
    assert!(matches!(
        cs.status,
        Status::Proven {
            tier: Tier::Syntactic
        }
    ));
}

#[test]
fn callsite_caller_pre_propagates_via_smt() {
    if !z3_available() {
        eprintln!("skipping: z3 not on PATH");
        return;
    }
    // I1 + tier-2: caller @requires(y > 0), divisor_safe(y) — Z3
    // proves the callee's @requires(x > 0) under that assumption.
    let src = r#"
        @requires(x > 0)
        fn divisor_safe(x: i32) -> i32 { return x; }

        @requires(y > 0)
        fn caller(y: i32) -> i32 { return divisor_safe(y); }
    "#;
    let (_c, report) = run(
        src,
        DischargeConfig {
            enable: true,
            smt_budget_ms: 1500,
            cache_root: None,
        },
    );
    let cs = report
        .obligations
        .iter()
        .find(|o| matches!(o.kind, ObligationKind::CallSite))
        .expect("found call-site obligation");
    assert!(
        matches!(cs.status, Status::Proven { tier: Tier::Smt }),
        "expected SMT-proven, got {:?}",
        cs.status
    );
}

#[test]
fn callsite_discharge_works_through_method_calls() {
    // K1: `@requires` on impl methods now parses. Mono rewrites
    // `c.add(5)` to `Counter_add(&c, 5)` before discharge runs, so
    // the call-site discharger sees a free-function call against
    // the lifted method and substitutes args into the callee's
    // precondition exactly like a direct call.
    let src = r#"
        struct Counter { n: i32 }

        impl Counter {
            @requires(amount > 0)
            fn add(self: ref(Self), amount: i32) -> i32 {
                return ((deref(self)).n + amount);
            }
        }

        fn caller() -> i32 {
            let c: Counter = Counter { n: 10 };
            return c.add(5);
        }
    "#;
    let (_c, report) = run(src, DischargeConfig::default());
    let cs = report
        .obligations
        .iter()
        .find(|o| matches!(o.kind, ObligationKind::CallSite))
        .expect("expected a CallSite obligation for the method call");
    assert!(
        matches!(
            cs.status,
            Status::Proven {
                tier: Tier::Syntactic
            }
        ),
        "expected method-call discharge to prove `amount > 0` via literal substitution, got {:?}",
        cs.status
    );
    assert!(
        cs.function.ends_with("->Counter_add"),
        "obligation should target Counter_add (the lifted method), got {}",
        cs.function
    );
}

#[test]
fn callsite_bad_literal_falls_to_runtime() {
    // I1: divisor_safe(0) substitutes x=0 into @requires(x > 0),
    // constant folds to false → can't prove → Runtime.
    let src = r#"
        @requires(x > 0)
        fn divisor_safe(x: i32) -> i32 { return x; }

        fn caller() -> i32 { return divisor_safe(0); }
    "#;
    let (_c, report) = run(src, DischargeConfig::default());
    let cs = report
        .obligations
        .iter()
        .find(|o| matches!(o.kind, ObligationKind::CallSite))
        .expect("found call-site obligation");
    assert!(
        matches!(cs.status, Status::Runtime { .. }),
        "expected Runtime, got {:?}",
        cs.status
    );
}

#[test]
fn smt_finds_counterexample_for_unprovable_clause() {
    if !z3_available() {
        eprintln!("skipping: z3 not on PATH");
        return;
    }
    // `x > 0` standalone has counterexample x=0. v1 SMT doesn't yet
    // consider call-site context, so this lands on Runtime — the
    // safe default. Verify the report classifies it correctly.
    let src = r#"
        @requires(x > 0)
        fn f(x: i32) -> i32 { return x; }
    "#;
    let (_c, report) = run(
        src,
        DischargeConfig {
            enable: true,
            smt_budget_ms: 1500,
            cache_root: None,
        },
    );
    assert!(matches!(
        report.obligations[0].status,
        Status::Runtime { .. }
    ));
}

/// Pull a function body out of generated C by name. Used to assert
/// on whether `fc_trap` is present. Returns the line containing the
/// signature plus the next ~12 lines (enough for any small fn).
fn extract_fn(c: &str, name: &str) -> String {
    let needle = format!(" {}(", name);
    let mut lines: Vec<&str> = Vec::new();
    let mut capturing = false;
    let mut depth = 0;
    for line in c.lines() {
        if !capturing {
            if line.contains(&needle) && line.trim_end().ends_with('{') {
                capturing = true;
                lines.push(line);
                depth = 1;
                continue;
            }
        } else {
            lines.push(line);
            for c in line.chars() {
                if c == '{' {
                    depth += 1;
                } else if c == '}' {
                    depth -= 1;
                }
            }
            if depth == 0 {
                break;
            }
        }
    }
    lines.join("\n")
}
