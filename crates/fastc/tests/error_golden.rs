//! Golden tests for error messages
//!
//! These tests verify that error messages are stable and informative.
//! Uses insta for snapshot testing.

use fastc::compile;

/// Test a file that should fail to compile and snapshot the error
fn test_error_snapshot(name: &str, source: &str) {
    let result = compile(source, &format!("{}.fc", name));
    assert!(result.is_err(), "Expected compilation to fail for {}", name);

    let error = result.unwrap_err();
    let error_str = format!("{:?}", error);

    insta::assert_snapshot!(name, error_str);
}

#[test]
fn test_error_parse() {
    test_error_snapshot("parse_error", include_str!("errors/parse_error.fc"));
}

#[test]
fn test_error_undefined_name() {
    test_error_snapshot("undefined_name", include_str!("errors/undefined_name.fc"));
}

#[test]
fn test_error_type_mismatch() {
    test_error_snapshot("type_mismatch", include_str!("errors/type_mismatch.fc"));
}

#[test]
fn test_error_unsafe_required() {
    test_error_snapshot("unsafe_required", include_str!("errors/unsafe_required.fc"));
}
