//! Stage-2.0 hardening: feed arbitrary bytes to `fastc::parse` and
//! assert no panics. Parse errors (the `Err` arm) are the expected
//! shape — anything else (panic, infinite loop caught by the
//! `-max_total_time` budget, abort) is a finding to fix.
//!
//! This harness only exercises the parser. Downstream passes
//! (resolve, typecheck, mono, lower) have their own integration
//! coverage via `tests/`; if the parser produces an AST cleanly,
//! they should also handle it. A separate `parse_to_check` target
//! exercising the full pipeline is a follow-up sub-slice once the
//! parser is fuzz-clean.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Reject non-UTF-8 inputs cheaply — the lexer assumes valid UTF-8
    // and would error on invalid bytes; the panic shape we care
    // about lives inside the recursive-descent parser, which we
    // need real UTF-8 to reach.
    let Ok(source) = std::str::from_utf8(data) else {
        return;
    };
    // 4 MB cap protects the fuzzer from spending all its budget on
    // a single giant input. The parser is roughly linear in source
    // size, so 4 MB is plenty to surface any quadratic blow-up bugs
    // (which would manifest as `-timeout=...` triggering libfuzzer
    // before reaching here).
    if source.len() > 4 * 1024 * 1024 {
        return;
    }

    // The result is intentionally discarded — we only care that
    // `parse` returns rather than panics. An `Err` is fine; an
    // unwind is a finding.
    let _ = fastc::parse(source, "fuzz.fc");
});
