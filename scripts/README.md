# scripts/

Top-level harness scripts for fastC contributors and CI.

| Script | Purpose | When to use |
|---|---|---|
| `test.sh` | Run the test suite | Local development |
| `test.sh quick` | Just the unit + integration tests | Fast feedback while iterating |
| `test.sh ci` | What CI runs (fmt + clippy + tests) | Mirror CI locally |
| `test.sh full` *(default)* | Everything: build + tests + format + examples smoke | Before pushing a branch |
| `check.sh` | Alias for `test.sh ci` | Pre-commit hook |
| `bench.sh` | Run the cross-language benchmark suite | Measuring perf changes |

## Quick start

```sh
# Fast: ~10 seconds
bash scripts/test.sh quick

# Pre-PR: ~30 seconds
bash scripts/test.sh ci

# Everything: ~2 minutes
bash scripts/test.sh

# Benchmarks: ~5 minutes
bash scripts/bench.sh
```

All scripts exit `0` on success, `1` on failure, `2` if a required
tool (cargo) is missing. They detect tty for color output, so
piping through `tee` or to a log file gives clean plaintext.

## What's tested

`test.sh quick` and `test.sh ci`:
- Every `crates/fastc/tests/*.rs` integration test (~330 tests)
- Every `#[test]` in `crates/fastc/src/`
- Format check (`cargo fmt --all --check`)

`test.sh full` (`ci` plus):
- Doc tests
- Examples smoke test — compiles 5 of the tutorial examples through
  `fastc compile` + `cc` end-to-end to catch regressions in the
  emit + runtime layer.

## Adding a new test surface

Tests live where Cargo expects them:
- Unit tests: `#[cfg(test)] mod tests` inside the relevant
  `crates/fastc/src/*.rs` file.
- Integration tests: a new `.rs` file under `crates/fastc/tests/`.
  Cargo picks them up automatically.
- Workspace-wide tests: any `cargo test --workspace` target.

The harness scripts don't need updating when you add a new test
file — they invoke `cargo test` which discovers everything.
