//! N4 (v2.0 closure captures): closures may reference outer
//! `let x = <literal>` bindings via constant inlining.
//!
//! v1 covers IntLit / BoolLit / FloatLit + unary-negated
//! literals. Non-literal captures (function results, struct
//! fields, mutable bindings) still emit the closure-aware
//! "undefined name" diagnostic with a fix-it pointing at the
//! v2.0 env-struct path.

use std::path::PathBuf;
use std::process::Command;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root")
        .to_path_buf()
}

fn fastc_release() -> PathBuf {
    workspace_root()
        .join("target")
        .join("release")
        .join("fastc")
}

fn compile_and_run(src: &str, label: &str) -> (i32, String) {
    let fastc = fastc_release();
    if !fastc.exists() {
        panic!("release fastc binary missing — run `cargo build --release -p fastc`");
    }
    let tmp = std::env::temp_dir().join(format!("fastc_closure_cap_{}", label));
    std::fs::create_dir_all(&tmp).unwrap();
    let src_path = tmp.join("input.fc");
    let c_path = tmp.join("out.c");
    let bin_path = tmp.join("bin");
    std::fs::write(&src_path, src).unwrap();

    let out = Command::new(&fastc)
        .args(["compile"])
        .arg(&src_path)
        .arg("-o")
        .arg(&c_path)
        .output()
        .expect("spawn fastc");
    assert!(
        out.status.success(),
        "fastc compile failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );

    let runtime = workspace_root().join("runtime");
    let cc_out = Command::new("cc")
        .arg(&c_path)
        .arg("-I")
        .arg(&runtime)
        .arg("-o")
        .arg(&bin_path)
        .output()
        .expect("spawn cc");
    assert!(
        cc_out.status.success(),
        "cc failed:\n{}",
        String::from_utf8_lossy(&cc_out.stderr)
    );

    let run = Command::new(&bin_path).output().expect("spawn binary");
    let code = run.status.code().unwrap_or(-1);
    (code, String::from_utf8_lossy(&run.stdout).into_owned())
}

#[test]
fn int_literal_capture_inlines_into_closure_body() {
    let src = r#"
        use io::print_int;
        use io::put_char;

        fn apply(f: fn(i32) -> i32, x: i32) -> i32 { return f(x); }

        fn main() -> i32 {
            let n: i32 = 5;
            let add_n: fn(i32) -> i32 = |x: i32| -> i32 { return (x + n); };
            let r: i32 = apply(add_n, 10);
            print_int(r);
            put_char(10);
            return 0;
        }
    "#;
    let (_code, out) = compile_and_run(src, "int_literal");
    assert!(
        out.contains("15"),
        "expected `apply(add_n, 10)` → 10+5 = 15, got stdout:\n{}",
        out
    );
}

#[test]
fn bool_literal_capture_inlines_into_if_condition() {
    // The capture lives inside a nested `if` — v1's inliner had to
    // gain Stmt::If recursion to handle this. Regression: when the
    // `if`/while branches aren't walked, the resolver rejects the
    // ident with the closure-aware error.
    let src = r#"
        use io::print_int;
        use io::put_char;

        fn apply(f: fn(i32) -> i32, x: i32) -> i32 { return f(x); }

        fn main() -> i32 {
            let flag: bool = true;
            let pick: fn(i32) -> i32 = |x: i32| -> i32 {
                if (flag) { return x; }
                return 0;
            };
            let r: i32 = apply(pick, 7);
            print_int(r);
            put_char(10);
            return 0;
        }
    "#;
    let (_code, out) = compile_and_run(src, "bool_in_if");
    assert!(
        out.contains("7"),
        "expected `apply(pick, 7)` with flag=true → 7, got stdout:\n{}",
        out
    );
}

#[test]
fn negative_literal_capture_inlines_correctly() {
    // -3 parses as a unary-negated IntLit. The classifier accepts it
    // as a literal init, and the closure body sees `x + (-3)`.
    let src = r#"
        use io::print_int;
        use io::put_char;

        fn apply(f: fn(i32) -> i32, x: i32) -> i32 { return f(x); }

        fn main() -> i32 {
            let bias: i32 = -3;
            let sub_3: fn(i32) -> i32 = |x: i32| -> i32 { return (x + bias); };
            let r: i32 = apply(sub_3, 10);
            print_int(r);
            put_char(10);
            return 0;
        }
    "#;
    let (_code, out) = compile_and_run(src, "neg_literal");
    assert!(
        out.contains("7"),
        "expected `apply(sub_3, 10)` → 10 + (-3) = 7, got stdout:\n{}",
        out
    );
}
