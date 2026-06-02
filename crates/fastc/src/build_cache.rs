//! Stage-1.7 follow-up (H4) — global build cache for emitted C.
//!
//! Hashes `(source_text, fastc_version, safety_level, target_triple)`
//! and stores the resulting C output under
//! `~/Library/Caches/fastc/build/<hex>.c` (or
//! `~/.cache/fastc/build/<hex>.c` on Linux). A subsequent compile
//! with the same inputs reads the cached C verbatim instead of
//! re-running the full lex → parse → resolve → typecheck → mono →
//! lower → emit pipeline.
//!
//! This is the on-disk persistence layer that complements the
//! in-process Salsa query cache from stage 0.8. Salsa gives us
//! "the same `fastc compile` invocation doesn't redo work";
//! `build_cache` gives us "two invocations against the same source
//! / version / target share work."
//!
//! ## What we do NOT cache
//!
//! - The header (`.h`) output. Cheap to regenerate; would add a
//!   second cache key for marginal savings.
//! - The discharge report. That's cached by the SMT layer itself
//!   under `.fastc/cache/discharge/`.
//! - The linked binary. That's a cc invocation, not a fastc one;
//!   caching it would conflate compiler caches.
//!
//! ## Why a separate disk layout from the discharge cache
//!
//! The discharge cache lives next to the *project* (`.fastc/cache/`)
//! so it ships with the source tree if a user wants to vendor it.
//! The build cache lives in `~/Library/Caches/` because it's a
//! machine-local performance store: the same `(source, version,
//! target)` triple compiles to the same C on any machine, but the
//! cache itself is not meant to be checked into a repo.

use std::path::PathBuf;

use crate::db::sha256;

/// What goes into the cache key. Identical keys produce identical
/// cached C, so any field that affects code generation must be here.
#[derive(Debug, Clone)]
pub struct CacheKey<'a> {
    pub source: &'a str,
    pub fastc_version: &'a str,
    pub safety_level: &'a str,
    pub target_triple: Option<&'a str>,
    pub emit_header: bool,
    /// `--strict` flag — affects whether warnings become errors,
    /// which can change whether a compile succeeds.
    pub strict: bool,
}

impl<'a> CacheKey<'a> {
    /// Hex-encoded SHA-256 of the key fields concatenated with `\0`
    /// separators so two distinct fields can't collide via
    /// concatenation.
    pub fn hash_hex(&self) -> String {
        let mut buf: Vec<u8> = Vec::with_capacity(self.source.len() + 128);
        buf.extend_from_slice(self.source.as_bytes());
        buf.push(0);
        buf.extend_from_slice(self.fastc_version.as_bytes());
        buf.push(0);
        buf.extend_from_slice(self.safety_level.as_bytes());
        buf.push(0);
        buf.extend_from_slice(self.target_triple.unwrap_or("native").as_bytes());
        buf.push(0);
        buf.push(if self.emit_header { 1 } else { 0 });
        buf.push(if self.strict { 1 } else { 0 });
        hex_encode(&sha256(&buf))
    }
}

/// Where the build cache lives. Returns `None` on platforms where
/// `dirs::cache_dir()` can't resolve a home (extremely rare —
/// containers running as uid 0 with no /root sometimes hit this).
fn cache_dir() -> Option<PathBuf> {
    Some(dirs::cache_dir()?.join("fastc").join("build"))
}

fn artifact_path(hex: &str) -> Option<PathBuf> {
    Some(cache_dir()?.join(format!("{}.c", hex)))
}

/// Look up the cached C output for `key`. Returns `Some(c_code)`
/// on hit, `None` on miss or read error.
pub fn lookup(key: &CacheKey<'_>) -> Option<String> {
    let path = artifact_path(&key.hash_hex())?;
    std::fs::read_to_string(path).ok()
}

/// Store the C output for `key`. Failures are silent — a build
/// that emits cleanly shouldn't fail just because the cache dir
/// is unwriteable.
pub fn store(key: &CacheKey<'_>, c_code: &str) {
    let Some(path) = artifact_path(&key.hash_hex()) else {
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(path, c_code);
}

/// M1 project-level cache. The path layout uses a separate
/// `project/` subdirectory so single-file and multi-file builds
/// don't collide on the same hash space. Each cached project key
/// gets two artifacts stored next to each other — `<hex>.c` and
/// `<hex>.h` (when a header was emitted).
fn project_dir() -> Option<PathBuf> {
    Some(dirs::cache_dir()?.join("fastc").join("project"))
}

fn project_paths(hex: &str) -> Option<(PathBuf, PathBuf)> {
    let dir = project_dir()?;
    Some((
        dir.join(format!("{}.c", hex)),
        dir.join(format!("{}.h", hex)),
    ))
}

/// Look up a cached multi-source build by project key. Returns
/// `Some((c_code, optional_header))` on hit; `None` on miss or
/// read error. The header is `None` when no `.h` was cached
/// alongside (the project didn't request `emit_header`).
pub fn lookup_project(project_key: &str) -> Option<(String, Option<String>)> {
    let (c_path, h_path) = project_paths(project_key)?;
    let c = std::fs::read_to_string(&c_path).ok()?;
    let h = std::fs::read_to_string(&h_path).ok();
    Some((c, h))
}

/// Store a successful build under the given project key. Failures
/// are swallowed silently so a permission problem on the cache
/// directory doesn't fail the user's build. Future lookups will
/// just see a miss and re-compile.
pub fn store_project(project_key: &str, c_code: &str, header: Option<&str>) {
    let Some((c_path, h_path)) = project_paths(project_key) else {
        return;
    };
    if let Some(parent) = c_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(&c_path, c_code);
    if let Some(h) = header {
        let _ = std::fs::write(&h_path, h);
    } else {
        let _ = std::fs::remove_file(&h_path);
    }
}

fn hex_encode(bytes: &[u8; 32]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(64);
    for &b in bytes.iter() {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0f) as usize] as char);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key_a() -> CacheKey<'static> {
        CacheKey {
            source: "fn main() -> i32 { return 0; }",
            fastc_version: "0.1.0",
            safety_level: "standard",
            target_triple: None,
            emit_header: false,
            strict: false,
        }
    }

    #[test]
    fn identical_keys_hash_identically() {
        let a = key_a();
        let b = key_a();
        assert_eq!(a.hash_hex(), b.hash_hex());
        assert_eq!(a.hash_hex().len(), 64);
    }

    #[test]
    fn different_source_different_hash() {
        let mut a = key_a();
        let mut b = key_a();
        a.source = "fn main() -> i32 { return 0; }";
        b.source = "fn main() -> i32 { return 1; }";
        assert_ne!(a.hash_hex(), b.hash_hex());
    }

    #[test]
    fn different_target_different_hash() {
        let mut a = key_a();
        let mut b = key_a();
        a.target_triple = None;
        b.target_triple = Some("aarch64-linux-musl");
        assert_ne!(a.hash_hex(), b.hash_hex());
    }

    #[test]
    fn different_safety_level_different_hash() {
        let mut a = key_a();
        let mut b = key_a();
        a.safety_level = "standard";
        b.safety_level = "critical";
        assert_ne!(a.hash_hex(), b.hash_hex());
    }

    #[test]
    fn store_then_lookup_round_trips() {
        let key = key_a();
        // Use a unique source so this test doesn't collide with parallel runs.
        let unique = format!("// build-cache-test-{}\n{}", std::process::id(), key.source);
        let mut k = key.clone();
        k.source = &unique;
        // Stash the C, read it back.
        store(&k, "int main(void) { return 0; }\n");
        let got = lookup(&k).expect("cache hit");
        assert!(got.contains("int main"));
    }
}
