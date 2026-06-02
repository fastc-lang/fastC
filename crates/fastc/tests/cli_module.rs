//! End-to-end test for `mod cli` — the stage 1.8 launch-set preview.
//!
//! Compiles `examples/cli_demo.fc` via the release binary, links it
//! with cc, and runs the resulting executable with a representative
//! argv. Validates that:
//!
//! - argv access via `cli::count` / `cli::arg_at` / `cli::program_name`
//!   round-trips the OS-passed argv array.
//! - `cli::has_flag` finds `--verbose` regardless of position.
//! - `cli::flag_value` accepts both `--name=value` and `--name value`.
//! - `cli::flag_int` parses base-10 integers and falls back when absent.
//! - The auto-generated `int main(int argc, char** argv)` wrapper
//!   correctly stashes argv via `fc_args_init`.
//!
//! Skipped when the release `fastc` binary isn't present (CI builds it
//! once before this test; locally `cargo build --release -p fastc`).

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

fn runtime_dir() -> PathBuf {
    workspace_root().join("runtime")
}

#[test]
fn cli_demo_round_trips_argv_and_flags() {
    let fastc = fastc_release();
    if !fastc.exists() {
        eprintln!("skipping: release fastc binary missing");
        return;
    }

    let tmp = std::env::temp_dir().join("fastc_cli_module_test");
    std::fs::create_dir_all(&tmp).expect("create tmp");
    let c_out = tmp.join("cli.c");
    let exe = tmp.join("cli");

    let demo = workspace_root().join("examples").join("cli_demo.fc");
    let out = Command::new(&fastc)
        .args(["compile"])
        .arg(&demo)
        .arg("-o")
        .arg(&c_out)
        .output()
        .expect("spawn fastc compile");
    assert!(
        out.status.success(),
        "fastc compile failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );

    let cc_out = Command::new("cc")
        .arg(&c_out)
        .arg("-I")
        .arg(runtime_dir())
        .arg("-o")
        .arg(&exe)
        .output()
        .expect("spawn cc");
    assert!(
        cc_out.status.success(),
        "cc failed:\n{}",
        String::from_utf8_lossy(&cc_out.stderr)
    );

    let run = Command::new(&exe)
        .args(["--name=Dipankar", "--count", "42", "--verbose", "trailing"])
        .output()
        .expect("spawn cli demo");
    assert!(
        run.status.success(),
        "cli demo exited non-zero:\n{}",
        String::from_utf8_lossy(&run.stderr)
    );

    let stdout = String::from_utf8_lossy(&run.stdout);
    assert!(
        stdout.contains("argc: 6"),
        "expected argc=6, got:\n{}",
        stdout
    );
    assert!(
        stdout.contains("--verbose? yes"),
        "expected --verbose detected:\n{}",
        stdout
    );
    assert!(
        stdout.contains("--name = Dipankar"),
        "expected --name=Dipankar parsed:\n{}",
        stdout
    );
    assert!(
        stdout.contains("--count = 42"),
        "expected --count=42 parsed:\n{}",
        stdout
    );
    assert!(
        stdout.contains("argv[5] = trailing"),
        "expected positional argv[5]:\n{}",
        stdout
    );
}

#[test]
fn cli_demo_uses_fallback_when_flag_absent() {
    let fastc = fastc_release();
    if !fastc.exists() {
        return;
    }
    let tmp = std::env::temp_dir().join("fastc_cli_module_test");
    let exe = tmp.join("cli");
    if !exe.exists() {
        // Previous test builds the binary; if it didn't run, skip.
        return;
    }

    let run = Command::new(&exe)
        .output()
        .expect("spawn cli demo with no flags");
    let stdout = String::from_utf8_lossy(&run.stdout);
    assert!(
        stdout.contains("--verbose? no"),
        "expected --verbose absent:\n{}",
        stdout
    );
    assert!(
        stdout.contains("--name = <absent>"),
        "expected --name absent:\n{}",
        stdout
    );
    assert!(
        stdout.contains("--count = 1"),
        "expected --count fallback=1:\n{}",
        stdout
    );
}
