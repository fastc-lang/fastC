//! L1 (stage-2.0 hardening): full compile pipeline fuzz target.
//!
//! Runs every byte sequence through `fastc::compile`, exercising
//! parse → resolve → typecheck → cap_check → noalloc_check → p10
//! → mono → lower → emit end-to-end. A panic anywhere is a
//! finding; the resulting C output is discarded.
//!
//! Per-iteration cost is higher than `check_no_panic` (lower +
//! emit are real work), so the libfuzzer budget shakes out fewer
//! corpus mutations per second. Useful for shaking out crashes in
//! the lower-pass invariants (CType resolution, struct topo-sort,
//! source-map line lookup) that the earlier passes can't reach.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(source) = std::str::from_utf8(data) else {
        return;
    };
    if source.len() > 4 * 1024 * 1024 {
        return;
    }
    let _ = fastc::compile(source, "fuzz.fc");
});
