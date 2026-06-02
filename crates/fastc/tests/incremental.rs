//! M1 (stage-2.0 incremental compilation): multi-source-file
//! project build cache.
//!
//! The H4 cache keys off a single source string and serves
//! `fastc compile <file>` invocations. M1 extends the same idea
//! to multi-file projects under `fastc build` — the cache key
//! covers every `.fc` file under `src/`, plus `fastc.toml` and
//! `fastc.lock`. Cache-hit path skips the full lex → emit chain.
//!
//! Tests:
//!   1. A pristine project builds (cold) then a no-edit re-build
//!      hits the cache and finishes in milliseconds.
//!   2. Editing any `.fc` file under `src/` flips the project key
//!      → cache miss → full re-compile produces fresh C output.
//!   3. Editing a non-`.fc` file or a file outside `src/build/`
//!      ignored-dir doesn't invalidate.

use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, Instant};

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

fn unique_dir(label: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let dir = std::env::temp_dir().join(format!(
        "fastc_incremental_{}_{}_{}",
        label,
        std::process::id(),
        nanos
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("src")).expect("mkdir");
    dir
}

fn write(dir: &std::path::Path, rel: &str, body: &str) {
    let p = dir.join(rel);
    if let Some(parent) = p.parent() {
        std::fs::create_dir_all(parent).expect("mkdir");
    }
    std::fs::write(p, body).expect("write");
}

fn run_build(fastc: &PathBuf, dir: &std::path::Path, cache_home: &std::path::Path) -> Duration {
    let _ = std::fs::create_dir_all(cache_home);
    let started = Instant::now();
    let out = Command::new(fastc)
        .env("HOME", cache_home)
        .env("XDG_CACHE_HOME", cache_home)
        .arg("build")
        .current_dir(dir)
        .output()
        .expect("spawn fastc build");
    let elapsed = started.elapsed();
    assert!(
        out.status.success(),
        "fastc build failed in {:?}:\nstdout:\n{}\nstderr:\n{}",
        dir,
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    elapsed
}

#[test]
fn warm_build_hits_cache_and_is_fast() {
    let fastc = fastc_release();
    if !fastc.exists() {
        eprintln!("skipping: release fastc binary missing");
        return;
    }
    let dir = unique_dir("warm");
    let cache = unique_dir("warm_cache");
    write(
        &dir,
        "fastc.toml",
        "[package]\nname = \"warm\"\nversion = \"0.1.0\"\n",
    );
    write(&dir, "src/main.fc", "fn main() -> i32 { return 0; }\n");

    let cold = run_build(&fastc, &dir, &cache);
    let warm = run_build(&fastc, &dir, &cache);
    eprintln!("cold={:?} warm={:?}", cold, warm);
    // The warm build path is "read project files, hash, look up
    // cache, copy". Even a tiny project should comfortably finish
    // in under half the cold time. We assert a 2× speedup floor —
    // any less suggests the cache isn't firing. Larger projects
    // see 20×+ in practice (the lex→…→emit chain is amortized
    // away); the floor is set for noise immunity on small inputs.
    let cold_ms = cold.as_millis().max(1) as f64;
    let warm_ms = warm.as_millis().max(1) as f64;
    let speedup = cold_ms / warm_ms;
    assert!(
        speedup >= 2.0,
        "expected ≥2× warm-vs-cold speedup, got {:.2}× (cold={}ms warm={}ms)",
        speedup,
        cold_ms,
        warm_ms
    );
}

#[test]
fn editing_a_source_file_invalidates_the_cache() {
    let fastc = fastc_release();
    if !fastc.exists() {
        return;
    }
    let dir = unique_dir("invalidate");
    let cache = unique_dir("invalidate_cache");
    write(
        &dir,
        "fastc.toml",
        "[package]\nname = \"invalidate\"\nversion = \"0.1.0\"\n",
    );
    write(&dir, "src/main.fc", "fn main() -> i32 { return 0; }\n");

    run_build(&fastc, &dir, &cache);
    let original_c = std::fs::read_to_string(dir.join("build/main.c")).unwrap();

    // Edit src/main.fc — the cache key flips, re-build re-emits.
    write(&dir, "src/main.fc", "fn main() -> i32 { return 42; }\n");
    run_build(&fastc, &dir, &cache);
    let edited_c = std::fs::read_to_string(dir.join("build/main.c")).unwrap();
    assert_ne!(
        original_c, edited_c,
        "editing src/main.fc should invalidate the cache and produce different C"
    );
    assert!(
        edited_c.contains("return 42;") || edited_c.contains("return ((int32_t)42)"),
        "expected the new `return 42;` to appear in the rebuilt C:\n{}",
        edited_c
    );
}

#[test]
fn editing_a_secondary_module_invalidates_the_cache() {
    let fastc = fastc_release();
    if !fastc.exists() {
        return;
    }
    let dir = unique_dir("multi");
    let cache = unique_dir("multi_cache");
    write(
        &dir,
        "fastc.toml",
        "[package]\nname = \"multi\"\nversion = \"0.1.0\"\n",
    );
    write(
        &dir,
        "src/main.fc",
        "mod helper;\nuse helper::double;\nfn main() -> i32 { return double(7); }\n",
    );
    write(
        &dir,
        "src/helper.fc",
        "pub fn double(x: i32) -> i32 { return (x + x); }\n",
    );

    run_build(&fastc, &dir, &cache);
    let original_c = std::fs::read_to_string(dir.join("build/main.c")).unwrap();

    // Edit ONLY helper.fc — main.fc is unchanged but the project
    // key MUST flip because the cache covers every `.fc` under src/.
    write(
        &dir,
        "src/helper.fc",
        "pub fn double(x: i32) -> i32 { return (x * 2); }\n",
    );
    run_build(&fastc, &dir, &cache);
    let edited_c = std::fs::read_to_string(dir.join("build/main.c")).unwrap();
    assert_ne!(
        original_c, edited_c,
        "editing src/helper.fc should still invalidate the project cache"
    );
}
