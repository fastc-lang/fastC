//! Deterministic output verification tests
//!
//! These tests verify that compiling the same source code produces
//! byte-identical output across multiple runs.

use fastc::compile;

/// Verify that compiling the same source twice produces identical output
fn verify_determinism(source: &str) {
    let output1 = compile(source, "test.fc").expect("First compilation failed");
    let output2 = compile(source, "test.fc").expect("Second compilation failed");
    assert_eq!(output1, output2, "Output must be deterministic across runs");
}

#[test]
fn test_determinism_simple_function() {
    verify_determinism(
        r#"
        fn main() -> i32 {
            return 0;
        }
        "#,
    );
}

#[test]
fn test_determinism_multiple_functions() {
    verify_determinism(
        r#"
        fn alpha() -> i32 { return 1; }
        fn beta() -> i32 { return 2; }
        fn gamma() -> i32 { return 3; }
        "#,
    );
}

#[test]
fn test_determinism_multiple_structs() {
    // Structs should be emitted in sorted order regardless of declaration order
    verify_determinism(
        r#"
        @repr(C)
        struct Zebra { x: i32, y: i32 }

        @repr(C)
        struct Alpha { a: i32, b: i32 }

        @repr(C)
        struct Middle { m: i32 }

        fn main() -> i32 { return 0; }
        "#,
    );
}

#[test]
fn test_determinism_multiple_enums() {
    // Enums should be emitted in sorted order
    verify_determinism(
        r#"
        enum Zebra { Z }
        enum Alpha { A }
        enum Middle { M }

        fn main() -> i32 { return 0; }
        "#,
    );
}

#[test]
fn test_determinism_mixed_types() {
    // Mix of structs and enums should have consistent ordering
    verify_determinism(
        r#"
        enum Zebra { Z }
        @repr(C)
        struct Alpha { x: i32 }
        enum Beta { B }
        @repr(C)
        struct Gamma { y: i32 }

        fn main() -> i32 { return 0; }
        "#,
    );
}

#[test]
fn test_determinism_opt_types() {
    // Generated opt types should be ordered deterministically
    verify_determinism(
        r#"
        fn use_opts() -> i32 {
            let x: opt(i32) = some(42);
            let y: opt(bool) = some(true);
            return 0;
        }
        "#,
    );
}

#[test]
fn test_determinism_complex_program() {
    // A more complex program with various features
    verify_determinism(
        r#"
        @repr(C)
        struct Point { x: i32, y: i32 }

        @repr(C)
        struct Vector { dx: i32, dy: i32 }

        enum Color { Red, Green, Blue }

        fn add(a: i32, b: i32) -> i32 {
            return a + b;
        }

        fn multiply(a: i32, b: i32) -> i32 {
            return a * b;
        }

        fn main() -> i32 {
            let x: i32 = add(1, 2);
            let y: i32 = multiply(3, 4);
            return x + y;
        }
        "#,
    );
}
