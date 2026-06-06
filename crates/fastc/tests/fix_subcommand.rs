//! C-phase fixit-backfill regression tests.
//!
//! Each test exercises `fastc fix` against a source that has a
//! specific deliberately-broken pattern. The fix should land, the
//! rewritten source should compile.

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
    workspace_root().join("target").join("release").join("fastc")
}

fn write_tmp(name: &str, body: &str) -> PathBuf {
    let dir = std::env::temp_dir().join("fastc_fix_tests");
    std::fs::create_dir_all(&dir).expect("mkdir");
    let p = dir.join(name);
    std::fs::write(&p, body).expect("write");
    p
}

#[test]
fn missing_semicolon_fixit_dry_run_shows_label() {
    let fastc = fastc_release();
    if !fastc.exists() {
        eprintln!("skipping: release fastc binary missing — `cargo build --release -p fastc`");
        return;
    }
    let src = "fn add(a: i32, b: i32) -> i32 {\n    return (a + b)\n}\n\nfn main() -> i32 {\n    return add(1, 2);\n}\n";
    let p = write_tmp("missing_semi_dry.fc", src);

    let out = Command::new(&fastc)
        .arg("fix")
        .arg(&p)
        .arg("--dry-run")
        .output()
        .expect("spawn");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "fastc fix --dry-run failed:\n{}\n{}",
        stdout,
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        stdout.contains("structured fixits applied"),
        "expected structured-fixit label, got:\n{}",
        stdout
    );
    assert!(
        stdout.contains("insert missing ';'"),
        "expected missing-';' label, got:\n{}",
        stdout
    );

    // Source on disk must be unchanged (--dry-run).
    let after = std::fs::read_to_string(&p).expect("read");
    assert_eq!(after, src, "source changed under --dry-run");
}

#[test]
fn missing_semicolon_fixit_applies_and_source_compiles() {
    let fastc = fastc_release();
    if !fastc.exists() {
        return;
    }
    let src = "fn add(a: i32, b: i32) -> i32 {\n    return (a + b)\n}\n\nfn main() -> i32 {\n    return add(1, 2);\n}\n";
    let p = write_tmp("missing_semi_apply.fc", src);

    let out = Command::new(&fastc)
        .arg("fix")
        .arg(&p)
        .output()
        .expect("spawn");
    assert!(
        out.status.success(),
        "fastc fix failed:\n{}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    // Source on disk should now contain the inserted semicolon.
    let after = std::fs::read_to_string(&p).expect("read");
    assert!(
        after.contains("return (a + b);"),
        "missing ';' was not inserted; source after fix:\n{}",
        after
    );

    // And the fixed source should now compile cleanly.
    let c_path = std::env::temp_dir().join("missing_semi_apply.c");
    let compile = Command::new(&fastc)
        .arg("compile")
        .arg(&p)
        .arg("-o")
        .arg(&c_path)
        .output()
        .expect("spawn compile");
    assert!(
        compile.status.success(),
        "post-fix source still doesn't compile:\n{}",
        String::from_utf8_lossy(&compile.stderr)
    );
}

#[test]
fn formatting_only_source_falls_through_to_fmt() {
    let fastc = fastc_release();
    if !fastc.exists() {
        return;
    }
    // No structural error — just bad formatting.
    let src = "fn  main()->i32{return 0;}";
    let p = write_tmp("fmt_only.fc", src);

    let out = Command::new(&fastc)
        .arg("fix")
        .arg(&p)
        .output()
        .expect("spawn");
    assert!(
        out.status.success(),
        "fastc fix failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let after = std::fs::read_to_string(&p).expect("read");
    assert!(
        after.contains("fn main()") && after.contains("-> i32"),
        "fmt didn't normalize the source:\n{}",
        after
    );
}
