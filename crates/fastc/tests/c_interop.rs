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
fn test_vec_compiles() {
    // Stage 1.1 slice 7: the `vec` module — first generic container.
    // Verifies struct-mono cooperates with generic free functions across
    // a non-trivial body that uses `sizeof(T)`, raw-pointer indexing via
    // `at(buf, i)`, and the new `addrm(x)` builtin for mutable refs.
    compile_and_verify("examples/vec_demo.fc");
}

#[test]
fn test_vec_ops_compiles() {
    // Stage 1.1 slice 9: `vec::pop` (returns `opt(T)`), `vec::clear`,
    // `vec::is_empty`, and `vec::contains[T: Eq]`. Exercises the
    // bounded-generic Eq dispatch path inside a generic container body.
    compile_and_verify("examples/vec_ops_demo.fc");
}

#[test]
fn test_str_compiles() {
    // Stage 1.1 slice 10: `Str` — owned byte string built on `Vec[u8]`.
    // First example of a non-generic struct holding a generic struct as
    // a field; verifies struct-mono picks up the embedded `Vec[u8]`
    // instantiation and that typedef topological ordering puts `Vec_u8`
    // before `Str` so C accepts the field declaration.
    compile_and_verify("examples/str_demo.fc");
}

#[test]
fn test_vec_map_compiles() {
    // Stage 1.1 slice 11: `vec::map[T, U]` — higher-order vec transform
    // taking a fn pointer. Drives type-arg inference through Fn-shape
    // unification: T comes from the receiver `Vec[T]`, U from the
    // mapping function's return type. Validates fn pointer + generic
    // interaction inside a generic container body.
    compile_and_verify("examples/vec_map_demo.fc");
}

#[test]
fn test_vec_higher_order_compiles() {
    // Stage 1.1 slice 12: `vec::swap` / `vec::reverse` (in-place
    // mutators) and `vec::filter[T]` (higher-order with predicate fn).
    // Reverse exercises the temp-slot swap pattern through raw pointers;
    // filter is the first stdlib API to build an empty Vec via direct
    // `alloc(0)` + struct literal (skipping `with_capacity`'s seed
    // requirement).
    compile_and_verify("examples/vec_higher_order_demo.fc");
}

#[test]
fn test_vec_sort_compiles() {
    // Stage 1.1 slice 13: `vec::sort[T: Ord]` — insertion sort using
    // the prelude Ord trait. Dispatches through
    // `cur.less_than(addr(prev))` which mono lowers to
    // `T_less_than(&cur, &prev)`. First bounded-generic mutator on the
    // container surface.
    compile_and_verify("examples/vec_sort_demo.fc");
}

#[test]
fn test_vec_for_each_compiles() {
    // Stage 1.1 slice 14: `vec::for_each[T](v, f: fn(T) -> void)`.
    // First stdlib API to take a void-returning fn pointer end-to-end.
    // Demo passes `io::put_char` directly, exercising both the
    // typedef pre-pass on `fn(i32) -> void` and Fn-shape inference
    // through a stdlib-provided receiver.
    compile_and_verify("examples/vec_for_each_demo.fc");
}

#[test]
fn test_vec_reduce_compiles() {
    // Stage 1.1 slice 15: `vec::reduce[T, U]` — left fold. First
    // stdlib API to take a two-argument fn pointer. Exercises typedef
    // synthesis on `fn(i32, i32) -> i32` and `unify_generic`'s Fn
    // recursion across both parameter positions.
    compile_and_verify("examples/vec_reduce_demo.fc");
}

#[test]
fn test_vec_extend_str_eq_compiles() {
    // Stage 1.1 slice 16: `vec::extend` (mod-internal generic-to-
    // generic at one more remove) + `str::eq` (byte-wise compare
    // reaching through `Str.data.data` to the embedded vec's raw
    // buffer, exercising the nested-Field projection added in
    // slice 10).
    compile_and_verify("examples/vec_extend_str_eq_demo.fc");
}

#[test]
fn test_hashmap_compiles() {
    // Stage 1.1 slice 18: `HashMap[K: Hash + Eq, V]` — first stdlib
    // type with two trait bounds on the same type parameter. Open-
    // addressing with linear probing; tombstones; growable via
    // rehash. Validates Hash trait dispatch alongside Eq dispatch in
    // a non-trivial generic container.
    compile_and_verify("examples/hashmap_demo.fc");
}

#[test]
fn test_io_format_compiles() {
    // Stage 1.1 slice 19: `io::print_int` + `str::write_line`. First
    // IO formatting helpers — print_int uses a new runtime
    // `fc_print_i32` that emits digits via putchar (no snprintf
    // dependency); write_line walks a Str's bytes via put_char
    // because Str isn't null-terminated.
    compile_and_verify("examples/io_format_demo.fc");
}

#[test]
fn test_vec_any_all_compiles() {
    // Stage 1.1 slice 20: `vec::any` + `vec::all` predicate scans
    // with short-circuit. Covers the vacuously-true case for `all`
    // and the vacuously-false case for `any` on an empty vec.
    // Slice 21 reverted the `evec` workaround back to `empty` after
    // lower's resolve_ident became scope-aware.
    compile_and_verify("examples/vec_any_all_demo.fc");
}

#[test]
fn test_vec_min_max_clone_compiles() {
    // Stage 1.1 slice 22: `vec::min[T:Ord]`, `vec::max[T:Ord]`,
    // `vec::clone[T]`, and `str::from_cstr`. Bounded-generic linear
    // scans for min/max (returning `opt(T)` to encode the empty
    // case); packed deep-copy for clone; first stdlib walk of an
    // FFI null-terminated `raw(u8)` for from_cstr.
    compile_and_verify("examples/vec_min_max_clone_demo.fc");
}

#[test]
fn test_str_helpers_compiles() {
    // Stage 1.1 slice 23: `str::starts_with` + `str::push_cstr`.
    // First stdlib prefix check and the natural sibling of from_cstr —
    // together they enable cheap string-building from C-string
    // fragments without allocating per-fragment.
    compile_and_verify("examples/str_helpers_demo.fc");
}

#[test]
fn test_hashmap_str_compiles() {
    // Stage 1.1 slice 24: `HashMap[Str, i32]` — first hashmap with a
    // non-primitive key. Powered by new `impl Hash for Str` (djb2)
    // and `impl Eq for Str` (byte compare). Validates the trait-
    // dispatch path on a user-defined struct type end-to-end through
    // the bounded-generic hashmap body.
    compile_and_verify("examples/hashmap_str_demo.fc");
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
