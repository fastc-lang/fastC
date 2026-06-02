//! End-to-end test for `mod http::get_status` against a local server.
//!
//! Spins up a Python `http.server` on a chosen port, compiles
//! `examples/http_demo.fc`, links and runs it, and verifies that the
//! status code 200 is returned and parsed correctly.
//!
//! Skipped when:
//! - the release `fastc` binary isn't present.
//! - `python3` isn't on PATH.
//! - the chosen port can't be bound (CI / sandbox).

use std::net::TcpStream;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Duration;

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

fn python3_available() -> bool {
    Command::new("python3")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn wait_for_port(port: u16) -> bool {
    for _ in 0..40 {
        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    false
}

#[test]
fn http_demo_gets_200_from_local_python_server() {
    let fastc = fastc_release();
    if !fastc.exists() {
        eprintln!("skipping: release fastc binary missing");
        return;
    }
    if !python3_available() {
        eprintln!("skipping: python3 not on PATH");
        return;
    }

    // Compile + link the demo first so we don't waste server time.
    let tmp = std::env::temp_dir().join("fastc_http_module_test");
    std::fs::create_dir_all(&tmp).expect("create tmp");
    let c_out = tmp.join("http.c");
    let exe = tmp.join("http");

    let demo = workspace_root().join("examples").join("http_demo.fc");
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
        .arg(workspace_root().join("runtime"))
        .arg("-o")
        .arg(&exe)
        .output()
        .expect("spawn cc");
    assert!(
        cc_out.status.success(),
        "cc failed:\n{}",
        String::from_utf8_lossy(&cc_out.stderr)
    );

    // The demo hardcodes port 8088; spin up python's HTTP server.
    let mut server = Command::new("python3")
        .args(["-m", "http.server", "8088", "--bind", "127.0.0.1"])
        .current_dir(&tmp)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn python3 http.server");

    if !wait_for_port(8088) {
        let _ = server.kill();
        eprintln!("skipping: couldn't bind 127.0.0.1:8088");
        return;
    }

    let run = Command::new(&exe).output().expect("spawn http demo");
    let _ = server.kill();
    let _ = server.wait();

    let stdout = String::from_utf8_lossy(&run.stdout);
    assert!(
        stdout.contains("status:") && stdout.contains("200"),
        "expected status 200, got:\n{}",
        stdout
    );
    assert_eq!(
        run.status.code(),
        Some(0),
        "expected exit 0 for 2xx, got {:?}",
        run.status.code()
    );
}
