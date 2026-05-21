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
fn test_generic_id_compiles() {
    // Stage 0.9: verify the monomorphization pass produces compilable C
    // for a program with two single-param and one multi-param generic
    // instantiation.
    compile_and_verify("examples/generic_id.fc");
}

#[test]
fn test_methods_compiles() {
    // Stage 1.0: inherent impl methods desugar to free `Type_method` C
    // functions and `p.method(args)` call sites rewrite with auto-address
    // of the receiver.
    compile_and_verify("examples/methods.fc");
}

#[test]
fn test_traits_compiles() {
    // Stage 1.0 slice 2: traits + impl-for + bounded generic with trait
    // method dispatch. Verifies mono specializes shout[T: Greeter] to
    // shout_Point and rewrites x.greet() to Point_greet(&x).
    compile_and_verify("examples/traits.fc");
}

#[test]
fn test_builtin_traits_compiles() {
    // Stage 1.0 slice 3: built-in Eq/Ord/Copy with primitive impls.
    // Verifies `fn max[T: Ord]` instantiates for both i32 and f64 using
    // the prelude-injected `i32_less_than` / `f64_less_than`.
    compile_and_verify("examples/builtin_traits.fc");
}

#[test]
fn test_drop_compiles() {
    // Stage 1.0 slice 4: Drop trait + compiler-generated scope-exit calls.
    // Verifies mono inserts Resource_drop(&a) and Resource_drop(&c) before
    // the function's return, in reverse declaration order, while leaving
    // Plain (no Drop impl) alone.
    compile_and_verify("examples/drop.fc");
}

#[test]
fn test_math_module_compiles() {
    // Stage 1.1 slice 1: math module shipped via the prelude. Bounded
    // generics (min/max/clamp on Ord) and primitive helpers (abs_i32 etc).
    compile_and_verify("examples/math_demo.fc");
}

#[test]
fn test_mem_module_compiles() {
    // Stage 1.1 slice 3: mem module wraps libc malloc/free via extern "C".
    compile_and_verify("examples/mem_demo.fc");
}

#[test]
fn test_io_module_compiles() {
    // Stage 1.1 slice 4: io module exposes `println` and `put_char` via
    // tiny runtime helpers that bridge `raw(u8)` to libc's `char*`.
    compile_and_verify("examples/io_demo.fc");
}

#[test]
fn test_fn_pointers_compile() {
    // Stage 1.1 slice 5: `fn(T) -> R` types are now first-class. Pass
    // named functions to higher-order helpers like `apply(f, x)`. The
    // emitter synthesizes typedefs (`fc_fn_int32_t_to_int32_t`) so C
    // declarations stay simple.
    compile_and_verify("examples/fn_ptr.fc");
}

#[test]
fn test_generic_struct_compiles() {
    // Stage 1.1 slice 6: generic structs. Mono specializes `Pair[A, B]`
    // per instantiation, emitting `Pair_i32_bool` / `Pair_f64_i32` as
    // independent typedefs.
    compile_and_verify("examples/generic_struct.fc");
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
