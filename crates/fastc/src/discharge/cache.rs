//! On-disk cache for stage-2.1 SMT discharge results.
//!
//! Z3 calls are deterministic on the SMT-LIB text we hand it (the
//! solver itself has internal nondeterminism but for the universal-
//! tautology + body-aware shapes we generate, the result for a
//! given input is stable). Caching by SMT-LIB-text hash lets a
//! re-run of `fastc compile --prove` reuse a previously-discharged
//! result in milliseconds instead of re-shelling z3.
//!
//! ## Layout
//!
//! ```text
//! .fastc/cache/discharge/
//!   <hex sha256 of SMT-LIB text>.bin
//! ```
//!
//! Each `.bin` is exactly one byte. The encoding is intentionally
//! tiny — no version header, no JSON, no length prefix — because
//! a cache miss is cheap (we'd shell z3 anyway) and the cache hit
//! path benefits from `read_to_string`-free I/O:
//!
//! - `b'P'` (0x50) — proven.
//! - `b'F'` (0x46) — z3 returned `sat` (counterexample). Falls to runtime.
//! - `b'T'` (0x54) — timeout / `unknown`.
//! - `b'U'` (0x55) — unsupported / encoder skipped.
//!
//! The cache is purely a performance optimization. A user can
//! `rm -rf .fastc/cache/discharge/` at any time; the worst that
//! happens is that the next build re-discharges every obligation.
//!
//! ## Invalidation
//!
//! There's no explicit invalidation. The key is the full SMT-LIB
//! text, so any change to the obligation expression, an assumption,
//! the body model, or the per-obligation budget produces a different
//! hash and therefore a different cache entry. Old entries pile up;
//! a future garbage-collection sub-slice can prune by mtime, but for
//! v1 we lean on the user (or `git clean -fdx`) to bound size.

use std::path::{Path, PathBuf};

use super::smt::SmtResult;
use crate::db::sha256;

/// Where the cache lives, relative to the project root.
fn cache_root(project_root: &Path) -> PathBuf {
    project_root.join(".fastc").join("cache").join("discharge")
}

/// Encode an SMT result as a single byte for on-disk storage.
fn encode(result: &SmtResult) -> u8 {
    match result {
        SmtResult::Proven => b'P',
        SmtResult::Failed(_) => b'F',
        SmtResult::Timeout => b'T',
        SmtResult::Unsupported(_) => b'U',
    }
}

/// Decode a stored byte. `None` if it's not one we recognize (corrupt
/// cache entry, future-version mismatch, …). The caller should treat
/// `None` as a cache miss and re-run the solver.
fn decode(byte: u8) -> Option<SmtResult> {
    match byte {
        b'P' => Some(SmtResult::Proven),
        // `Failed` / `Unsupported` carry a reason string — we don't
        // round-trip the reason; the cached "this was a failure"
        // bit is enough for the lower-pass behavior. The discharge
        // report renders a generic "cached: counterexample" reason.
        b'F' => Some(SmtResult::Failed(
            "cached: Z3 found a counterexample".to_string(),
        )),
        b'T' => Some(SmtResult::Timeout),
        b'U' => Some(SmtResult::Unsupported("cached: unsupported".to_string())),
        _ => None,
    }
}

/// Hex-encode 32 bytes to a 64-char filename-safe string.
fn hex_encode(bytes: &[u8; 32]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(64);
    for &b in bytes.iter() {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0f) as usize] as char);
    }
    out
}

/// Filesystem path for a given SMT-LIB text. The hash is the cache
/// key; users with a hash collision are doing something deeply
/// unusual.
fn path_for(project_root: &Path, smt_lib: &str) -> PathBuf {
    let digest = sha256(smt_lib.as_bytes());
    let hex = hex_encode(&digest);
    cache_root(project_root).join(format!("{}.bin", hex))
}

/// Look up a cached result. Returns `None` on miss or on a read
/// error (treated identically — the caller will just re-run Z3).
pub fn lookup(project_root: &Path, smt_lib: &str) -> Option<SmtResult> {
    let path = path_for(project_root, smt_lib);
    let bytes = std::fs::read(path).ok()?;
    if bytes.is_empty() {
        return None;
    }
    decode(bytes[0])
}

/// Record a result. Failures are swallowed silently — a permission
/// problem on `.fastc/` shouldn't fail the build. Future cache
/// lookups will just see a miss and reprove.
pub fn store(project_root: &Path, smt_lib: &str, result: &SmtResult) {
    let path = path_for(project_root, smt_lib);
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(&path, [encode(result)]);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_proven() {
        let dir = std::env::temp_dir().join("fastc_discharge_cache_test");
        let _ = std::fs::remove_dir_all(&dir);
        let smt = "(set-logic QF_LIA)\n(check-sat)";
        assert!(lookup(&dir, smt).is_none(), "fresh cache is empty");
        store(&dir, smt, &SmtResult::Proven);
        let got = lookup(&dir, smt).expect("cached value");
        assert!(matches!(got, SmtResult::Proven));
    }

    #[test]
    fn different_smt_text_different_entries() {
        let dir = std::env::temp_dir().join("fastc_discharge_cache_diff");
        let _ = std::fs::remove_dir_all(&dir);
        store(&dir, "a", &SmtResult::Proven);
        store(&dir, "b", &SmtResult::Timeout);
        assert!(matches!(lookup(&dir, "a"), Some(SmtResult::Proven)));
        assert!(matches!(lookup(&dir, "b"), Some(SmtResult::Timeout)));
    }

    #[test]
    fn corrupt_byte_is_a_cache_miss() {
        let dir = std::env::temp_dir().join("fastc_discharge_cache_corrupt");
        let _ = std::fs::remove_dir_all(&dir);
        let smt = "anything";
        let path = path_for(&dir, smt);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, [0xFFu8]).unwrap();
        assert!(lookup(&dir, smt).is_none(), "unrecognized byte → miss");
    }

    #[test]
    fn store_failure_is_silent() {
        // Pointing the project root at a non-writable parent should
        // not panic. The discharge pipeline keeps running.
        let bogus = std::path::PathBuf::from("/dev/null/no/such/place");
        store(&bogus, "x", &SmtResult::Proven);
    }
}
