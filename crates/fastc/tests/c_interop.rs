//! C interop tests
//!
//! These tests verify that the generated C code compiles with a C11 compiler.

use std::path::PathBuf;
use std::process;
use tempfile::tempdir;

/// Get the workspace root directory
fn workspace_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // Go up two levels from crates/fastc to workspace root
    manifest_dir
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

/// Check if clang is available
fn clang_available() -> bool {
    process::Command::new("clang")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Compile a FastC file and verify the generated C compiles
fn compile_and_verify(fc_file: &str) {
    if !clang_available() {
        eprintln!("Skipping test: clang not available");
        return;
    }

    let root = workspace_root();
    let fc_path = root.join(fc_file);
    let runtime_path = root.join("runtime");

    let dir = tempdir().expect("Failed to create temp dir");
    let c_file = dir.path().join("output.c");

    // Run fastc to compile the .fc file
    let fastc_bin = env!("CARGO_BIN_EXE_fastc");
    let output = process::Command::new(fastc_bin)
        .arg("compile")
        .arg(&fc_path)
        .arg("-o")
        .arg(&c_file)
        .output()
        .expect("Failed to run fastc");
    assert!(
        output.status.success(),
        "fastc failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Compile with clang
    let output = process::Command::new("clang")
        .args(["-std=c11", "-Wall", "-Werror", "-c"])
        .arg(&c_file)
        .arg("-I")
        .arg(&runtime_path)
        .output()
        .expect("Failed to run clang");

    if !output.status.success() {
        eprintln!("clang stderr: {}", String::from_utf8_lossy(&output.stderr));
        panic!("C compilation failed for {}", fc_file);
    }
}

#[test]
fn test_hello_compiles() {
    compile_and_verify("examples/hello.fc");
}

#[test]
fn test_pointers_compile() {
    compile_and_verify("examples/pointers.fc");
}

#[test]
fn test_opt_example_compiles() {
    compile_and_verify("examples/opt_example.fc");
}

#[test]
fn test_if_let_example_compiles() {
    compile_and_verify("examples/if_let_example.fc");
}

#[test]
fn test_slice_example_compiles() {
    compile_and_verify("examples/slice_example.fc");
}

#[test]
fn test_overflow_example_compiles() {
    compile_and_verify("examples/overflow_example.fc");
}

#[test]
fn test_enum_example_compiles() {
    compile_and_verify("examples/enum_example.fc");
}

#[test]
fn test_switch_example_compiles() {
    compile_and_verify("examples/switch_example.fc");
}

#[test]
fn test_interop_types_compiles() {
    compile_and_verify("examples/interop_types.fc");
}

/// Compile and run a C interop test that verifies ABI layout
fn run_interop_test(c_test_file: &str) {
    if !clang_available() {
        eprintln!("Skipping test: clang not available");
        return;
    }

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let c_test_path = manifest_dir.join("tests/interop").join(c_test_file);

    let dir = tempdir().expect("Failed to create temp dir");
    let exe_file = dir.path().join("test_exe");

    // Compile the C test to an executable
    let output = process::Command::new("clang")
        .args(["-std=c11", "-Wall", "-Werror"])
        .arg(&c_test_path)
        .arg("-o")
        .arg(&exe_file)
        .output()
        .expect("Failed to run clang");

    if !output.status.success() {
        eprintln!("clang stderr: {}", String::from_utf8_lossy(&output.stderr));
        panic!("C compilation failed for {}", c_test_file);
    }

    // Run the test
    let output = process::Command::new(&exe_file)
        .output()
        .expect("Failed to run test");

    if !output.status.success() {
        eprintln!("Test stderr: {}", String::from_utf8_lossy(&output.stderr));
        panic!(
            "Interop test failed for {}: exit code {:?}",
            c_test_file,
            output.status.code()
        );
    }
}

#[test]
fn test_struct_layout() {
    run_interop_test("layout_test.c");
}

#[test]
fn test_enum_layout() {
    run_interop_test("enum_test.c");
}

#[test]
fn test_slice_layout() {
    run_interop_test("slice_test.c");
}
