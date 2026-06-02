//! L1 (stage-2.0 hardening): contract-discharge pipeline fuzz
//! target.
//!
//! Drives random byte sequences through `fastc::parse` → the
//! `discharge_file` pass with the default config (tier-1 only, no
//! z3 spawn). Exercises the obligation collector, syntactic
//! discharger, SMT encoder build-path (without invoking z3), and
//! the report JSON serializer. A panic anywhere is a finding;
//! parse-error inputs are skipped (they short-circuit before
//! discharge runs).
//!
//! Distinct from `check_no_panic` because that target stops at
//! `check`, which doesn't invoke the discharge pass. SMT-tier
//! coverage stays out of fuzz because spawning z3 per iteration
//! would make per-second throughput unworkable.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(source) = std::str::from_utf8(data) else {
        return;
    };
    if source.len() > 4 * 1024 * 1024 {
        return;
    }
    let Ok(ast) = fastc::parse(source, "fuzz.fc") else {
        return;
    };
    let report = fastc::discharge::discharge_file(
        &ast,
        &fastc::discharge::DischargeConfig::default(),
    );
    // Exercise the JSON serializer too — it walks every obligation
    // and the escape path, which is its own surface for panics
    // (encode/escape bugs on weird identifier bytes).
    let _ = report.to_json();
});
