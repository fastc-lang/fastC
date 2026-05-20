//! Per-pass compile-time instrumentation.
//!
//! Captures elapsed wall-clock time for each phase of the compiler pipeline
//! (lex, parse, resolve, typecheck, p10, lower, emit). Emitted as JSON when
//! the user passes `--timing`, and consumed by `fastc bench` against
//! `compile-time-budget.toml`.
//!
//! See `docs/compile-time-budget.md` for the methodology this module
//! implements.

use std::cell::RefCell;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

/// One pass's timing record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PassTiming {
    /// Pass name, e.g. "lex", "parse", "typecheck".
    pub pass: String,
    /// Elapsed wall-clock time in milliseconds.
    pub ms: u64,
    /// Salsa cache status. "miss" until the query system lands; "hit" / "miss"
    /// after stage 0.8 Salsa migration.
    pub cache: CacheStatus,
}

/// Cache hit/miss status for a pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CacheStatus {
    /// Pass ran end-to-end; no cache.
    Miss,
    /// Result served from Salsa cache.
    Hit,
    /// Cache not applicable to this pass.
    Na,
}

/// A full timing record for one compilation invocation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimingReport {
    /// Input file path.
    pub input: String,
    /// Total elapsed time across all passes (ms).
    pub total_ms: u64,
    /// Per-pass timings, in execution order.
    pub passes: Vec<PassTiming>,
    /// Salsa cache hit count summed across passes.
    pub salsa_cache_hits: u32,
    /// Salsa cache miss count summed across passes.
    pub salsa_cache_misses: u32,
}

impl TimingReport {
    /// Render the report as pretty-printed JSON.
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    /// Render the report as a human-readable markdown table.
    pub fn to_markdown(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!("# Timing: {}\n\n", self.input));
        out.push_str(&format!("Total: **{}ms**\n\n", self.total_ms));
        out.push_str("| Pass | ms | Cache |\n");
        out.push_str("|------|---:|:-----:|\n");
        for p in &self.passes {
            let cache_str = match p.cache {
                CacheStatus::Miss => "miss",
                CacheStatus::Hit => "hit",
                CacheStatus::Na => "n/a",
            };
            out.push_str(&format!("| {} | {} | {} |\n", p.pass, p.ms, cache_str));
        }
        out.push_str(&format!(
            "\nSalsa hits: {}, misses: {}\n",
            self.salsa_cache_hits, self.salsa_cache_misses
        ));
        out
    }
}

/// Builder for a `TimingReport`. Thread-local during a compilation; pass
/// boundaries are recorded by `pass(...)` scopes.
pub struct TimingBuilder {
    input: String,
    started_at: Instant,
    passes: Vec<PassTiming>,
}

impl TimingBuilder {
    /// Start a new timing builder for the given input file.
    pub fn new(input: impl Into<String>) -> Self {
        Self {
            input: input.into(),
            started_at: Instant::now(),
            passes: Vec::new(),
        }
    }

    /// Record a pass that completed in `elapsed`.
    pub fn record(&mut self, pass: impl Into<String>, elapsed: Duration, cache: CacheStatus) {
        self.passes.push(PassTiming {
            pass: pass.into(),
            ms: elapsed.as_millis() as u64,
            cache,
        });
    }

    /// Finalize and produce the report.
    pub fn finish(self) -> TimingReport {
        let total = self.started_at.elapsed().as_millis() as u64;
        let hits = self
            .passes
            .iter()
            .filter(|p| p.cache == CacheStatus::Hit)
            .count() as u32;
        let misses = self
            .passes
            .iter()
            .filter(|p| p.cache == CacheStatus::Miss)
            .count() as u32;
        TimingReport {
            input: self.input,
            total_ms: total,
            passes: self.passes,
            salsa_cache_hits: hits,
            salsa_cache_misses: misses,
        }
    }
}

thread_local! {
    /// Active timing builder for the current compilation, if any.
    static CURRENT: RefCell<Option<TimingBuilder>> = const { RefCell::new(None) };
}

/// Install a fresh `TimingBuilder` for the current thread, replacing any
/// previous one. Used by the driver entry points when `--timing` is on.
pub fn install(input: impl Into<String>) {
    CURRENT.with(|cell| *cell.borrow_mut() = Some(TimingBuilder::new(input)));
}

/// Remove and return the current thread's `TimingReport`, if any was
/// installed. Returns `None` when no timing was being recorded.
pub fn take() -> Option<TimingReport> {
    CURRENT.with(|cell| cell.borrow_mut().take().map(|b| b.finish()))
}

/// Run `body` with `pass` timing recorded into the current builder. If no
/// builder is installed, this is a zero-overhead pass-through.
pub fn time_pass<R>(pass: &'static str, body: impl FnOnce() -> R) -> R {
    let active = CURRENT.with(|cell| cell.borrow().is_some());
    if !active {
        return body();
    }
    let start = Instant::now();
    let result = body();
    let elapsed = start.elapsed();
    CURRENT.with(|cell| {
        if let Some(b) = cell.borrow_mut().as_mut() {
            b.record(pass, elapsed, CacheStatus::Miss);
        }
    });
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timing_pass_recorded() {
        install("test.fc");
        time_pass("lex", || std::thread::sleep(Duration::from_millis(1)));
        time_pass("parse", || std::thread::sleep(Duration::from_millis(1)));
        let report = take().expect("timing should be installed");
        assert_eq!(report.passes.len(), 2);
        assert_eq!(report.passes[0].pass, "lex");
        assert_eq!(report.passes[1].pass, "parse");
        assert!(report.total_ms >= 2);
    }

    #[test]
    fn timing_inactive_passthrough() {
        // No `install()` — pass-through must not panic.
        let val = time_pass("lex", || 42);
        assert_eq!(val, 42);
        assert!(take().is_none());
    }

    #[test]
    fn json_round_trip() {
        let report = TimingReport {
            input: "test.fc".into(),
            total_ms: 100,
            passes: vec![PassTiming {
                pass: "lex".into(),
                ms: 50,
                cache: CacheStatus::Miss,
            }],
            salsa_cache_hits: 0,
            salsa_cache_misses: 1,
        };
        let json = report.to_json();
        let parsed: TimingReport = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.total_ms, 100);
        assert_eq!(parsed.passes.len(), 1);
    }
}
