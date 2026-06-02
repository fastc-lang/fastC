# fastc-fuzz — parser fuzz harness

Stage-2.0 hardening item, shipped as part of the v1.x follow-ups.

## Quick start

```sh
# One-time setup (nightly toolchain + cargo-fuzz binary).
rustup install nightly
cargo install cargo-fuzz

# Run with a 5-minute budget.
cd crates/fastc/fuzz
cargo +nightly fuzz run parse_no_panic -- -max_total_time=300
```

Seeds for the corpus go in `corpus/parse_no_panic/`. To populate
from the in-repo examples:

```sh
mkdir -p corpus/parse_no_panic
cp ../../../examples/*.fc corpus/parse_no_panic/
```

Then re-run the harness — libfuzzer will pick the new seeds up
automatically and use them as starting points for mutation.

## Targets

- **`parse_no_panic`** — feeds arbitrary UTF-8 bytes to
  `fastc::parse` and asserts no panics. Parse errors are the
  expected shape; anything else is a finding to fix.

Future targets (each gets its own `fuzz_targets/*.rs`):

- `lex_no_panic` — same as above but only the lexer (faster
  per-iteration, useful for narrowing token-level regressions).
- `parse_to_check` — full pipeline through `fastc::check`. Once the
  parser is fuzz-clean, expand coverage downward.

## CI integration

`.github/workflows/fuzz.yml` runs a 5-minute fuzz pass on every PR
that touches `crates/fastc/src/lexer/` or `crates/fastc/src/parser/`.
A finding fails the workflow and uploads the reproducing input as
a build artifact for the author to triage.

## Reproducing a finding

When libfuzzer finds a crash, it drops the reproducing input under
`artifacts/parse_no_panic/crash-…`. To reproduce locally:

```sh
cargo +nightly fuzz run parse_no_panic artifacts/parse_no_panic/crash-abc123
```

The harness re-runs the exact bytes and panics in the same place,
giving you a stack trace under whatever `RUST_BACKTRACE` setting
you're using.
