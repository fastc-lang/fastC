//! Stage 1.3 function-level annotation tests.
//!
//! Covers `@mem(arena=...)`, `@panics(never|always|on=...)`,
//! `@purity(pure|effect|io)`, and `@complexity(O(...))`.
//! Documentation-only annotations parse and surface through `fastc
//! explain` JSON; the two enforced annotations (`@panics(never)`,
//! `@purity(pure)`) reject violations.

use fastc::compile;

#[test]
fn purity_pure_accepts_arithmetic_only_fn() {
    let src = r#"
        @purity(pure)
        fn square(x: i32) -> i32 {
            return (x * x);
        }
        fn main() -> i32 {
            return square(5);
        }
    "#;
    assert!(
        compile(src, "ok.fc").is_ok(),
        "pure arithmetic fn was rejected"
    );
}

#[test]
fn purity_pure_rejects_fn_that_logs() {
    let src = r#"
        use io::println;
        @purity(pure)
        fn impure() -> i32 {
            println(cstr("hi"));
            return 0;
        }
        fn main() -> i32 {
            return impure();
        }
    "#;
    let err = compile(src, "bad.fc").unwrap_err();
    let msg = format!("{:?}", err);
    assert!(
        msg.contains("@purity(pure)") && msg.contains("println"),
        "expected @purity(pure)/println diagnostic, got: {}",
        msg
    );
}

#[test]
fn purity_pure_rejects_fn_that_allocates() {
    let src = r#"
        use mem::alloc;
        @purity(pure)
        fn alloc_inside() -> rawm(u8) {
            return alloc(cast(usize, 16));
        }
        fn main() -> i32 {
            discard(alloc_inside());
            return 0;
        }
    "#;
    let err = compile(src, "bad.fc").unwrap_err();
    let msg = format!("{:?}", err);
    assert!(
        msg.contains("@purity(pure)"),
        "expected @purity(pure) diagnostic, got: {}",
        msg
    );
}

#[test]
fn panics_never_accepts_pure_arithmetic() {
    let src = r#"
        @panics(never)
        fn add(a: i32, b: i32) -> i32 {
            return (a + b);
        }
        fn main() -> i32 {
            return add(1, 2);
        }
    "#;
    assert!(
        compile(src, "ok.fc").is_ok(),
        "@panics(never) arithmetic fn was rejected"
    );
}

#[test]
fn purity_effect_and_io_are_documentation_only_and_compile() {
    // Neither @purity(effect) nor @purity(io) trigger a banned-set
    // check — they're documentation in v1.x. Compile end-to-end to
    // prove the parser + AST + driver all flow through cleanly.
    let src = r#"
        use io::println;
        @purity(effect)
        fn maybe_mutates(x: i32) -> i32 {
            return (x + 1);
        }
        @purity(io)
        fn does_io() -> i32 {
            println(cstr("running"));
            return 0;
        }
        fn main() -> i32 {
            let a: i32 = maybe_mutates(5);
            let b: i32 = does_io();
            return (a + b);
        }
    "#;
    let r = compile(src, "doc_only.fc");
    assert!(
        r.is_ok(),
        "@purity(effect)/@purity(io) should compile, err: {:?}",
        r.unwrap_err()
    );
}

#[test]
fn mem_arena_is_documentation_only_and_parses() {
    let src = r#"
        @mem(arena = scratch)
        fn from_scratch(x: i32) -> i32 {
            return (x * 2);
        }
        fn main() -> i32 {
            return from_scratch(7);
        }
    "#;
    assert!(
        compile(src, "mem.fc").is_ok(),
        "@mem(arena=...) should parse + compile (documentation-only)"
    );
}

#[test]
fn complexity_parses_each_bigo_shape() {
    // O(1), O(n), O(log n), O(n log n), O(n^2), O(2^n) — every
    // shape the parser DSL accepts. None of them affect compilation
    // (documentation-only) but they all need to round-trip.
    let src = r#"
        @complexity(O(1))
        fn constant() -> i32 { return 0; }
        @complexity(O(n))
        fn linear(x: i32) -> i32 { return x; }
        @complexity(O(log n))
        fn logarithmic(x: i32) -> i32 { return x; }
        @complexity(O(n log n))
        fn linearithmic(x: i32) -> i32 { return x; }
        @complexity(O(n^2))
        fn quadratic(x: i32) -> i32 { return (x * x); }
        @complexity(O(2^n))
        fn exponential(x: i32) -> i32 { return x; }
        fn main() -> i32 {
            return ((((((constant() + linear(1)) + logarithmic(2)) + linearithmic(3)) + quadratic(4)) + exponential(5)));
        }
    "#;
    let r = compile(src, "complexity.fc");
    assert!(
        r.is_ok(),
        "all @complexity shapes should parse + compile, err: {:?}",
        r.unwrap_err()
    );
}

#[test]
fn duplicate_annotation_is_rejected() {
    let src = r#"
        @purity(pure)
        @purity(io)
        fn conflicted() -> i32 { return 0; }
        fn main() -> i32 { return conflicted(); }
    "#;
    let err = compile(src, "dup.fc").unwrap_err();
    let msg = format!("{:?}", err);
    assert!(
        msg.to_lowercase().contains("duplicate") && msg.contains("@purity"),
        "expected duplicate-@purity diagnostic, got: {}",
        msg
    );
}
