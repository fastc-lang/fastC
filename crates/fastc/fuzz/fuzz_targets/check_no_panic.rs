//! L1 (stage-2.0 hardening): full typecheck pipeline fuzz target.
//!
//! Where `parse_no_panic` exercises only the lexer + parser, this
//! target runs every byte sequence through `fastc::check`, which
//! threads the input through resolve, typecheck, cap_check,
//! noalloc_check, and the Power-of-10 checker. A panic anywhere
//! in that chain is a finding.
//!
//! Parse errors are the expected shape for most random input;
//! anything that survives parsing exercises the analysis passes,
//! which is the value of having a separate target from `parse_no_panic`.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(source) = std::str::from_utf8(data) else {
        return;
    };
    if source.len() > 4 * 1024 * 1024 {
        return;
    }
    // `check` is the cheapest entry that exercises every analysis
    // pass — it stops before mono/lower/emit, which keeps each
    // iteration fast enough for libfuzzer's per-second throughput
    // budget. The `_` discards the parse / resolve / typecheck
    // diagnostic stream; we only care that no panic escapes.
    let _ = fastc::check(source, "fuzz.fc");
});
