//! Salsa-style query database skeleton for incremental compilation.
//!
//! This is a minimal hand-rolled implementation of the query/memoization
//! pattern that Salsa provides. It exists to land the *architecture* — every
//! compiler pass becomes a pure function of its hashed inputs, cached by
//! that hash — without yet pulling in the full Salsa crate. Migrating to the
//! real Salsa crate is scheduled for stage 2.0; this skeleton lets stages
//! 0.8–1.x layer caching onto individual passes incrementally.
//!
//! Design properties this skeleton commits to:
//!
//! - **Pure-function queries.** Every query is `(input-hash) -> output`. No
//!   ambient state, no side effects. Re-running with the same input must
//!   produce the same output.
//! - **Hash-based invalidation.** Inputs are SHA-256-hashed; the hash is the
//!   cache key. Salsa's red-green algorithm is *not* implemented here yet —
//!   the skeleton has no dependency tracking between queries. That lands
//!   with the real Salsa migration.
//! - **One query implemented end-to-end.** `tokens(source)` proves the
//!   plumbing. Adding more queries is a matter of declaring them on `Db`
//!   following the same pattern.
//!
//! See `docs/compile-time-budget.md` (stage 0.8) for the larger plan.
//!
//! ## Module-level parallelism (deferred)
//!
//! The roadmap's stage 0.8 bullet for "module-level parallelism (rayon)" is
//! intentionally *not* implemented in this skeleton. The current compile
//! pipeline runs a single file through one linear pass; module expansion
//! inlines all modules into one AST before lowering. There is no
//! independent unit of work to parallelize.
//!
//! The opportunity opens up at stage 0.9 (monomorphization, where each
//! generic instantiation is independent) and again at stage 1.1 (when the
//! stdlib creates a real multi-module call graph). Adding rayon now would
//! pay the compile-time cost of the dependency without measurable speedup —
//! exactly the kind of regression the `compile-time-budget.toml` gate exists
//! to prevent.
//!
//! When the parallel slice exists, the `Db` lock is already a `Mutex`
//! precisely so it can be shared across worker threads.

use std::collections::HashMap;
use std::sync::Mutex;

use crate::lexer::{Lexer, Spanned, Token};

/// One lexer output element. The actual token plus its span.
pub type SpannedToken = Spanned<Token>;

/// Compiler query database. Owns memoized results keyed by input hash.
///
/// One `Db` instance per compilation session. The intent is that the build
/// driver creates one `Db`, threads it through every pass, and queries
/// against it. Cache hits short-circuit recomputation.
#[derive(Default)]
pub struct Db {
    // Inner state lives behind a `Mutex` because the build driver may
    // dispatch passes onto a `rayon` worker pool (stage 0.8 module-level
    // parallelism). Contention is low — passes spend nearly all their time
    // computing, not contending on the cache.
    inner: Mutex<DbInner>,
}

#[derive(Default)]
struct DbInner {
    /// Lex query results keyed by SHA-256 of the source text.
    lex_cache: HashMap<InputHash, Vec<SpannedToken>>,
    /// Counters for diagnostic reporting and the `--timing` integration.
    hits: u64,
    misses: u64,
}

// --- Planned but not yet implemented ---
//
// Stages A5b/c of the compile-time roadmap call for these query slots:
//   parse_cache:     HashMap<InputHash, Arc<crate::ast::File>>
//   resolve_cache:   HashMap<InputHash, Arc<SymbolTable>>
//   typecheck_cache: HashMap<InputHash, Arc<TypeCheckResult>>
//
// All four (lex + the three above) want an on-disk counterpart at
// `.fastc/cache/queries/<query>/<hash>.bin`. The disk cache is the
// value-producing slice — it survives across CLI invocations, turning
// cold compiles into warm compiles for unchanged source.
//
// Held-back work, in dependency order:
//   1. AST refactor to share storage via Arc<...> without per-call clones
//      that empirically regress single-shot CLI compile (see the A1
//      prelude-caching experiment in commit history for the measurement).
//   2. Serialize / Deserialize derives on every AST node + bincode for
//      the on-disk format.
//   3. Per-query cache directory layout under .fastc/cache/queries/<name>/.
//   4. Salsa-style red/green dependency tracking so a change to one input
//      invalidates only downstream queries, not the whole cache.
//   5. The fastc-daemon (stage A6) becomes the primary consumer of the
//      hot in-memory cache; the CLI consumes the cold on-disk cache.

/// SHA-256 hash of a query's input bytes. 32 bytes, fixed-size, cheap to
/// hash and compare. Kept as its own type so a query that accidentally hands
/// `String` to another query won't typecheck.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct InputHash(pub [u8; 32]);

impl InputHash {
    /// Compute the hash of `bytes`.
    pub fn of(bytes: &[u8]) -> Self {
        Self(sha256(bytes))
    }
}

/// Lightweight SHA-256 implementation. Hand-rolled to avoid pulling in the
/// `sha2` crate; this code path is not perf-critical (it runs once per query
/// input) and the algorithm is small and well-known.
fn sha256(bytes: &[u8]) -> [u8; 32] {
    // Initial hash values: first 32 bits of fractional parts of sqrt of first 8 primes.
    const H0: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    // Round constants: first 32 bits of fractional parts of cube roots of first 64 primes.
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];

    let mut h = H0;
    let bit_len: u64 = (bytes.len() as u64) * 8;

    // Build padded message: bytes ++ 0x80 ++ zeros ++ bit_len(big-endian).
    let mut msg: Vec<u8> = bytes.to_vec();
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bit_len.to_be_bytes());

    for chunk in msg.chunks_exact(64) {
        let mut w = [0u32; 64];
        for (i, word) in chunk.chunks_exact(4).enumerate() {
            w[i] = u32::from_be_bytes([word[0], word[1], word[2], word[3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let mut state = h;
        for i in 0..64 {
            let s1 =
                state[4].rotate_right(6) ^ state[4].rotate_right(11) ^ state[4].rotate_right(25);
            let ch = (state[4] & state[5]) ^ (!state[4] & state[6]);
            let temp1 = state[7]
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 =
                state[0].rotate_right(2) ^ state[0].rotate_right(13) ^ state[0].rotate_right(22);
            let maj = (state[0] & state[1]) ^ (state[0] & state[2]) ^ (state[1] & state[2]);
            let temp2 = s0.wrapping_add(maj);
            state[7] = state[6];
            state[6] = state[5];
            state[5] = state[4];
            state[4] = state[3].wrapping_add(temp1);
            state[3] = state[2];
            state[2] = state[1];
            state[1] = state[0];
            state[0] = temp1.wrapping_add(temp2);
        }
        for i in 0..8 {
            h[i] = h[i].wrapping_add(state[i]);
        }
    }

    let mut out = [0u8; 32];
    for (i, word) in h.iter().enumerate() {
        out[i * 4..i * 4 + 4].copy_from_slice(&word.to_be_bytes());
    }
    out
}

impl Db {
    /// Construct a fresh, empty query database.
    pub fn new() -> Self {
        Self::default()
    }

    /// Lex query: source text → token stream. Cached by SHA-256 of the
    /// source. Subsequent calls with identical source return the cached
    /// vector without re-running logos.
    ///
    /// This is the proof-of-concept query that exercises the cache. Other
    /// passes (parse, resolve, typecheck, …) will be added following the
    /// same shape as stage 0.8 progresses.
    pub fn tokens(&self, source: &str) -> Vec<SpannedToken> {
        let hash = InputHash::of(source.as_bytes());
        // Clone-or-miss avoids holding overlapping borrows on `inner`.
        {
            let mut inner = self.inner.lock().unwrap();
            let cached = inner.lex_cache.get(&hash).cloned();
            if let Some(toks) = cached {
                inner.hits += 1;
                return toks;
            }
        }
        // Cache miss: run the lex pass.
        let tokens: Vec<SpannedToken> = Lexer::new(source).collect();
        let mut inner = self.inner.lock().unwrap();
        inner.misses += 1;
        inner.lex_cache.insert(hash, tokens.clone());
        tokens
    }

    /// Total cache hits across all queries.
    pub fn hits(&self) -> u64 {
        self.inner.lock().unwrap().hits
    }

    /// Total cache misses across all queries.
    pub fn misses(&self) -> u64 {
        self.inner.lock().unwrap().misses
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_miss_then_hit() {
        let db = Db::new();
        let src = "fn main() -> i32 { return 0; }";
        let toks1 = db.tokens(src);
        assert_eq!(db.misses(), 1);
        assert_eq!(db.hits(), 0);

        let toks2 = db.tokens(src);
        assert_eq!(db.misses(), 1);
        assert_eq!(db.hits(), 1);
        assert_eq!(toks1.len(), toks2.len());
    }

    #[test]
    fn different_source_misses() {
        let db = Db::new();
        let _ = db.tokens("fn a() -> i32 { return 0; }");
        let _ = db.tokens("fn b() -> i32 { return 1; }");
        assert_eq!(db.misses(), 2);
        assert_eq!(db.hits(), 0);
    }

    #[test]
    fn sha256_known_vector() {
        // Empty string vector from RFC 6234.
        let expected = [
            0xe3, 0xb0, 0xc4, 0x42, 0x98, 0xfc, 0x1c, 0x14, 0x9a, 0xfb, 0xf4, 0xc8, 0x99, 0x6f,
            0xb9, 0x24, 0x27, 0xae, 0x41, 0xe4, 0x64, 0x9b, 0x93, 0x4c, 0xa4, 0x95, 0x99, 0x1b,
            0x78, 0x52, 0xb8, 0x55,
        ];
        assert_eq!(sha256(b""), expected);
    }

    #[test]
    fn sha256_abc() {
        // "abc" vector from RFC 6234.
        let expected = [
            0xba, 0x78, 0x16, 0xbf, 0x8f, 0x01, 0xcf, 0xea, 0x41, 0x41, 0x40, 0xde, 0x5d, 0xae,
            0x22, 0x23, 0xb0, 0x03, 0x61, 0xa3, 0x96, 0x17, 0x7a, 0x9c, 0xb4, 0x10, 0xff, 0x61,
            0xf2, 0x00, 0x15, 0xad,
        ];
        assert_eq!(sha256(b"abc"), expected);
    }
}
