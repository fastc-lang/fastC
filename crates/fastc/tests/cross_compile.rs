//! End-to-end cross-compile tests.
//!
//! Each target compiles `examples/hello.fc` via the release `fastc` binary
//! and inspects the produced file's magic bytes to confirm the right
//! architecture / ABI was emitted. The test:
//!
//! - Requires `zig` on PATH; skips with a printed note when absent so CI
//!   runners without zig still pass.
//! - Requires the release `fastc` binary at `target/release/fastc`;
//!   builds it implicitly by depending on the crate, but in practice CI
//!   builds it once via `cargo build --release -p fastc` before this test.
//! - Runs in a per-target temp directory under `/tmp/fastc_xc_<triple>/`
//!   so concurrent test runs (different targets) don't stomp each other.

use std::path::{Path, PathBuf};
use std::process::Command;

fn zig_available() -> bool {
    let Some(path_env) = std::env::var_os("PATH") else {
        return false;
    };
    for dir in std::env::split_paths(&path_env) {
        if dir.join("zig").is_file() {
            return true;
        }
    }
    false
}

fn workspace_root() -> PathBuf {
    // crates/fastc/tests/cross_compile.rs → workspace root is two ../ up
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest_dir)
        .parent() // crates/
        .and_then(|p| p.parent()) // workspace root
        .expect("workspace root")
        .to_path_buf()
}

fn fastc_release_binary() -> PathBuf {
    workspace_root()
        .join("target")
        .join("release")
        .join("fastc")
}

fn scaffold_tmp_project(triple: &str) -> PathBuf {
    let tmp = std::env::temp_dir().join(format!("fastc_xc_{}", triple));
    let _ = std::fs::remove_dir_all(&tmp);
    let src_dir = tmp.join("src");
    std::fs::create_dir_all(&src_dir).expect("create src dir");

    let hello = workspace_root().join("examples").join("hello.fc");
    let main_fc = src_dir.join("main.fc");
    std::fs::copy(&hello, &main_fc).expect("copy hello.fc");

    std::fs::write(
        tmp.join("fastc.toml"),
        "[package]\nname = \"hello_xc\"\nversion = \"0.1.0\"\n",
    )
    .expect("write fastc.toml");

    tmp
}

fn build_for(triple: &str) -> Option<PathBuf> {
    if !zig_available() {
        eprintln!("skipping {triple}: zig not on PATH");
        return None;
    }
    let fastc = fastc_release_binary();
    if !fastc.exists() {
        eprintln!(
            "skipping {triple}: release fastc binary missing at {}. \
            Run `cargo build --release -p fastc` first.",
            fastc.display()
        );
        return None;
    }
    let project = scaffold_tmp_project(triple);
    let output = Command::new(&fastc)
        .args(["build", "--target"])
        .arg(triple)
        .current_dir(&project)
        .output()
        .expect("spawn fastc");
    assert!(
        output.status.success(),
        "fastc build --target={} failed:\nstdout:\n{}\nstderr:\n{}",
        triple,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );

    let build_dir = project.join("build");
    // wasm32-wasi produces main.wasm; everything else produces `main`.
    let wasm = build_dir.join("main.wasm");
    if wasm.exists() {
        return Some(wasm);
    }
    let elf = build_dir.join("main");
    assert!(
        elf.exists(),
        "expected build/main or build/main.wasm to exist for {}",
        triple
    );
    Some(elf)
}

fn read_first_bytes(path: &Path, n: usize) -> Vec<u8> {
    use std::io::Read;
    let mut f = std::fs::File::open(path).expect("open binary");
    let mut buf = vec![0u8; n];
    let read = f.read(&mut buf).expect("read binary");
    buf.truncate(read);
    buf
}

/// ELF magic: 0x7F 'E' 'L' 'F'
fn is_elf(path: &Path) -> bool {
    let b = read_first_bytes(path, 4);
    b == [0x7F, b'E', b'L', b'F']
}

/// Mach-O 64-bit magic: 0xCFFAEDFE (LE on disk) or 0xFEEDFACF (BE-formatted).
/// On modern macOS the on-disk layout starts with 0xCF 0xFA 0xED 0xFE.
fn is_macho_64(path: &Path) -> bool {
    let b = read_first_bytes(path, 4);
    b == [0xCF, 0xFA, 0xED, 0xFE] || b == [0xFE, 0xED, 0xFA, 0xCF]
}

/// WASM magic: \0asm 0x01 0x00 0x00 0x00
fn is_wasm(path: &Path) -> bool {
    let b = read_first_bytes(path, 8);
    b == [0x00, b'a', b's', b'm', 0x01, 0x00, 0x00, 0x00]
}

#[test]
fn aarch64_linux_musl_produces_elf() {
    let Some(p) = build_for("aarch64-linux-musl") else {
        return;
    };
    assert!(is_elf(&p), "expected ELF magic for aarch64-linux-musl");
}

#[test]
fn x86_64_linux_musl_produces_elf() {
    let Some(p) = build_for("x86_64-linux-musl") else {
        return;
    };
    assert!(is_elf(&p), "expected ELF magic for x86_64-linux-musl");
}

#[test]
fn aarch64_linux_gnu_produces_elf() {
    let Some(p) = build_for("aarch64-linux-gnu") else {
        return;
    };
    assert!(is_elf(&p), "expected ELF magic for aarch64-linux-gnu");
}

#[test]
fn x86_64_linux_gnu_produces_elf() {
    let Some(p) = build_for("x86_64-linux-gnu") else {
        return;
    };
    assert!(is_elf(&p), "expected ELF magic for x86_64-linux-gnu");
}

#[test]
fn aarch64_macos_produces_macho() {
    let Some(p) = build_for("aarch64-macos") else {
        return;
    };
    assert!(is_macho_64(&p), "expected Mach-O magic for aarch64-macos");
}

#[test]
fn x86_64_macos_produces_macho() {
    let Some(p) = build_for("x86_64-macos") else {
        return;
    };
    assert!(is_macho_64(&p), "expected Mach-O magic for x86_64-macos");
}

#[test]
fn wasm32_wasi_produces_wasm() {
    let Some(p) = build_for("wasm32-wasi") else {
        return;
    };
    assert!(is_wasm(&p), "expected WASM magic for wasm32-wasi");
    assert!(
        p.extension().and_then(|e| e.to_str()) == Some("wasm"),
        "wasm32-wasi output should have .wasm extension, got {:?}",
        p
    );
}

#[test]
fn riscv64_linux_musl_produces_elf() {
    let Some(p) = build_for("riscv64-linux-musl") else {
        return;
    };
    assert!(is_elf(&p), "expected ELF magic for riscv64-linux-musl");
}

#[test]
fn unknown_target_errors_clearly() {
    let fastc = fastc_release_binary();
    if !fastc.exists() {
        eprintln!("skipping: release fastc binary missing");
        return;
    }
    let project = scaffold_tmp_project("unknown");
    let output = Command::new(&fastc)
        .args(["build", "--target=mips32-aix-r6"])
        .current_dir(&project)
        .output()
        .expect("spawn fastc");
    assert!(
        !output.status.success(),
        "fastc should reject unknown triple"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("unknown target") && stderr.contains("fastc target list"),
        "expected helpful error pointing at `fastc target list`; got:\n{}",
        stderr
    );
}
