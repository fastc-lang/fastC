//! B3 inline `test { }` block tests.

use fastc::compile;

#[test]
fn test_block_compiles_and_test_fns_are_stripped() {
    // The test{} block parses, each contained fn gets is_test=true,
    // and the driver strips them from the non-test build so they
    // don't appear in the emitted C.
    let src = r#"
        test {
            fn check_arithmetic() -> i32 {
                return ((2 + 2) - 4);
            }
            fn check_other() -> i32 {
                return 0;
            }
        }
        fn main() -> i32 {
            return 0;
        }
    "#;
    let r = compile(src, "tests.fc");
    assert!(r.is_ok(), "test{{}} block should compile: {:?}", r);
    let c = r.unwrap();
    // Test fns are stripped from the non-test build — the emitted
    // C must not mention their names.
    assert!(
        !c.contains("check_arithmetic"),
        "test fn 'check_arithmetic' leaked into normal build:\n{}",
        c
    );
    assert!(
        !c.contains("check_other"),
        "test fn 'check_other' leaked into normal build:\n{}",
        c
    );
}

#[test]
fn user_fn_named_test_still_works() {
    // `fn test()` is a perfectly valid user function — the contextual
    // keyword only hijacks `test {` (test followed by an open brace).
    let src = r#"
        fn test() -> i32 {
            return 7;
        }
        fn main() -> i32 {
            return test();
        }
    "#;
    let r = compile(src, "user_test.fc");
    assert!(
        r.is_ok(),
        "user fn named 'test' should still parse: {:?}",
        r
    );
}

#[test]
fn non_fn_inside_test_block_is_rejected() {
    let src = r#"
        test {
            struct Bad { x: i32 }
        }
        fn main() -> i32 { return 0; }
    "#;
    let r = compile(src, "bad.fc");
    assert!(
        r.is_err(),
        "non-fn inside test{{}} block should be rejected"
    );
}
