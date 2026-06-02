//! Stage 1.3 module-level header tests.
//!
//! Tests the lenient enforcement mode (default for back-compat):
//! - `mod foo { ... }` without `//!` parses fine.
//! - `mod foo { ... }` with `//!` must have the complete required-key set.
//! - Cross-module: `@owns` uniqueness, `@depends` exhaustiveness,
//!   `@arch` layering.

use fastc::compile;

#[test]
fn legacy_module_without_header_compiles() {
    // A bare `mod foo { ... }` block with no `//!` lines is a v1.x
    // legacy module — parses and compiles untouched.
    let src = r#"
        mod legacy {
            pub fn helper(x: i32) -> i32 {
                return (x + 1);
            }
        }
        fn main() -> i32 {
            return legacy::helper(0);
        }
    "#;
    let r = compile(src, "legacy.fc");
    assert!(r.is_ok(), "legacy module rejected: {:?}", r.unwrap_err());
}

#[test]
fn complete_module_header_compiles() {
    let src = r#"
        mod tested {
            //! @module = "tested"
            //! @owns = "tested"
            //! @arch = "core"
            //! @depends = ""
            //! @threading = "single"
            //! @invariants = "non-negative"
            pub fn double(x: i32) -> i32 {
                return (x + x);
            }
        }
        fn main() -> i32 {
            return tested::double(3);
        }
    "#;
    let r = compile(src, "complete.fc");
    assert!(
        r.is_ok(),
        "complete module header rejected: {:?}",
        r.unwrap_err()
    );
}

#[test]
fn partial_header_is_rejected() {
    // Module has a `//!` line but is missing required keys.
    let src = r#"
        mod partial {
            //! @module = "partial"
            //! @owns = "partial"
            pub fn id(x: i32) -> i32 { return x; }
        }
        fn main() -> i32 {
            return partial::id(0);
        }
    "#;
    let err = compile(src, "partial.fc").unwrap_err();
    let msg = format!("{:?}", err);
    assert!(
        msg.contains("partial header") || msg.contains("missing"),
        "expected partial-header diagnostic, got: {}",
        msg
    );
}

#[test]
fn duplicate_owns_namespace_is_rejected() {
    let src = r#"
        mod alpha {
            //! @module = "alpha"
            //! @owns = "shared"
            //! @arch = "core"
            //! @depends = ""
            //! @threading = "single"
            //! @invariants = "ok"
            pub fn one() -> i32 { return 1; }
        }
        mod beta {
            //! @module = "beta"
            //! @owns = "shared"
            //! @arch = "core"
            //! @depends = ""
            //! @threading = "single"
            //! @invariants = "ok"
            pub fn two() -> i32 { return 2; }
        }
        fn main() -> i32 {
            return ((alpha::one()) + (beta::two()));
        }
    "#;
    let err = compile(src, "dup_owns.fc").unwrap_err();
    let msg = format!("{:?}", err);
    assert!(
        msg.contains("@owns") && msg.contains("shared"),
        "expected @owns/'shared' duplicate diagnostic, got: {}",
        msg
    );
}

#[test]
fn arch_layering_rejects_low_to_high() {
    // bottom is declared first → rank 0. top is declared second →
    // rank 1. bottom depends on top — a lower layer depending on a
    // higher one violates the DAG rule.
    let src = r#"
        mod bottom {
            //! @module = "bottom"
            //! @owns = "bottom"
            //! @arch = "lower"
            //! @depends = "top"
            //! @threading = "single"
            //! @invariants = "ok"
            use top::ping;
            pub fn call_top() -> i32 { return ping(); }
        }
        mod top {
            //! @module = "top"
            //! @owns = "top"
            //! @arch = "upper"
            //! @depends = ""
            //! @threading = "single"
            //! @invariants = "ok"
            pub fn ping() -> i32 { return 1; }
        }
        fn main() -> i32 {
            return bottom::call_top();
        }
    "#;
    let err = compile(src, "arch_layers.fc").unwrap_err();
    let msg = format!("{:?}", err);
    assert!(
        msg.contains("layering"),
        "expected arch-layering diagnostic, got: {}",
        msg
    );
}
