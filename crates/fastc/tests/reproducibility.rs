//! L2 (stage-2.0 hardening): cross-directory reproducibility.
//!
//! The J1/J2 source-map directives embed the source path in
//! `#line N "<file>"` directives. Without normalization, the same
//! `.fc` source produces different C bytes depending on the
//! absolute path it was compiled from — defeating content-hash
//! caches and reproducible-build verification.
//!
//! `fastc compile --reproducible` normalizes the embedded path to
//! the basename so two compilations in different directories
//! produce byte-identical C. This test verifies the property
//! end-to-end against the release binary.

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

fn write_source(dir: &PathBuf, name: &str, body: &str) -> PathBuf {
    std::fs::create_dir_all(dir).expect("mkdir");
    let p = dir.join(name);
    std::fs::write(&p, body).expect("write source");
    p
}

#[test]
fn same_source_in_different_dirs_produces_identical_c_under_reproducible() {
    let fastc = fastc_release();
    if !fastc.exists() {
        eprintln!("skipping: release fastc binary missing — run `cargo build --release -p fastc`");
        return;
    }

    let body = "fn first(x: i32) -> i32 { return (x + 1); }\n\
                fn main() -> i32 { return first(5); }\n";
    let dir_a = std::env::temp_dir().join("fastc_repro_a");
    let dir_b = std::env::temp_dir().join("fastc_repro_b");
    let src_a = write_source(&dir_a, "hello.fc", body);
    let src_b = write_source(&dir_b, "hello.fc", body);
    let out_a = dir_a.join("out.c");
    let out_b = dir_b.join("out.c");

    // The H4 global build cache keys off the source bytes — if we
    // don't isolate it, both runs share the same cached output
    // regardless of path normalization. Use a temp HOME so each run
    // gets its own cache root and we genuinely re-compile.
    fn compile_clean(fastc: &PathBuf, src: &PathBuf, out: &PathBuf, reproducible: bool) {
        let cache_root = std::env::temp_dir().join(format!(
            "fastc_repro_cache_{}",
            std::process::id() ^ (out.to_string_lossy().len() as u32)
        ));
        let _ = std::fs::remove_dir_all(&cache_root);
        std::fs::create_dir_all(&cache_root).unwrap();
        let mut cmd = Command::new(fastc);
        cmd.env("HOME", &cache_root)
            .env("XDG_CACHE_HOME", &cache_root)
            .arg("compile")
            .arg(src)
            .arg("-o")
            .arg(out);
        if reproducible {
            cmd.arg("--reproducible");
        }
        let result = cmd.output().expect("spawn fastc");
        assert!(
            result.status.success(),
            "fastc compile failed:\n{}",
            String::from_utf8_lossy(&result.stderr)
        );
    }

    // Without --reproducible the absolute path leaks into `#line`
    // directives — bytes diverge between the two dirs.
    compile_clean(&fastc, &src_a, &out_a, false);
    compile_clean(&fastc, &src_b, &out_b, false);
    let bytes_a = std::fs::read(&out_a).unwrap();
    let bytes_b = std::fs::read(&out_b).unwrap();
    assert_ne!(
        bytes_a, bytes_b,
        "without --reproducible, C output should differ across paths (sanity)"
    );

    // With --reproducible the bytes match exactly.
    compile_clean(&fastc, &src_a, &out_a, true);
    compile_clean(&fastc, &src_b, &out_b, true);
    let r_a = std::fs::read(&out_a).unwrap();
    let r_b = std::fs::read(&out_b).unwrap();
    assert_eq!(
        r_a, r_b,
        "--reproducible should produce byte-identical C across paths"
    );

    // The normalized #line directives carry only the basename.
    let r_a_str = std::str::from_utf8(&r_a).expect("utf8");
    assert!(
        r_a_str.contains("#line 1 \"hello.fc\""),
        "expected basename-only #line directive in reproducible output"
    );
    assert!(
        !r_a_str.contains("/tmp/fastc_repro_a"),
        "absolute path leaked into reproducible output:\n{}",
        r_a_str
    );
}

/// C2 (v1.x supply-chain polish): a multi-file project that pulls
/// in a vendored dep produces byte-identical C across clean
/// builds in different working directories.
///
/// Uses a tiny fixture: a project with src/main.fc + src/helper.fc
/// in two separate temp dirs. Both build with --reproducible and
/// the C output should be byte-identical even though the build
/// roots differ. This verifies the property at the dep / multi-
/// file level, not just the single-file level that the test above
/// covers.
#[test]
fn multi_file_project_reproducible_across_dirs() {
    let fastc = fastc_release();
    if !fastc.exists() {
        eprintln!("skipping: release fastc binary missing");
        return;
    }

    let toml = "[package]\nname = \"dep_repro\"\nversion = \"0.1.0\"\n";
    let main_fc = "mod helper;\nuse helper::double;\nfn main() -> i32 { return double(21); }\n";
    let helper_fc = "pub fn double(x: i32) -> i32 { return (x + x); }\n";

    let dir_a = std::env::temp_dir().join("fastc_dep_repro_a");
    let dir_b = std::env::temp_dir().join("fastc_dep_repro_b");
    let _ = std::fs::remove_dir_all(&dir_a);
    let _ = std::fs::remove_dir_all(&dir_b);
    for dir in [&dir_a, &dir_b] {
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::write(dir.join("fastc.toml"), toml).unwrap();
        std::fs::write(dir.join("src/main.fc"), main_fc).unwrap();
        std::fs::write(dir.join("src/helper.fc"), helper_fc).unwrap();
    }

    fn build_clean(fastc: &PathBuf, dir: &PathBuf) -> PathBuf {
        // Isolated cache root per dir so we don't share the M1
        // project cache (which would mask any path divergence).
        let cache_root = std::env::temp_dir().join(format!(
            "fastc_dep_repro_cache_{}",
            std::process::id() ^ (dir.to_string_lossy().len() as u32)
        ));
        let _ = std::fs::remove_dir_all(&cache_root);
        std::fs::create_dir_all(&cache_root).unwrap();
        let out = Command::new(fastc)
            .env("HOME", &cache_root)
            .env("XDG_CACHE_HOME", &cache_root)
            .arg("build")
            .arg("--reproducible")
            .current_dir(dir)
            .output()
            .expect("spawn fastc build");
        assert!(
            out.status.success(),
            "build failed in {}:\n{}",
            dir.display(),
            String::from_utf8_lossy(&out.stderr)
        );
        dir.join("build").join("main.c")
    }

    let c_a = build_clean(&fastc, &dir_a);
    let c_b = build_clean(&fastc, &dir_b);
    let bytes_a = std::fs::read(&c_a).expect("read a");
    let bytes_b = std::fs::read(&c_b).expect("read b");
    assert_eq!(
        bytes_a, bytes_b,
        "multi-file project should produce byte-identical C across working dirs under --reproducible"
    );
}
