# Testing

fastC ships a one-command test harness at `scripts/test.sh`. The
script wraps `cargo test`, format checks, doc tests, and an examples
smoke test through a single entry point with colorized output and
clear pass/fail counts.

You should never need to remember which `cargo` incantation matches
which level of "is my change OK to push?" — pick a mode below and
the harness handles the rest.

## Quick start

```bash
bash scripts/test.sh quick    # ~20s — unit + integration only
bash scripts/test.sh ci       # mirror what CI runs
bash scripts/test.sh          # full — everything
```

All three modes exit `0` on success, `1` on failure, and `2` if
`cargo` itself isn't installed. They detect TTY for colorized
output, so piping through `tee` or to a log file gives clean
plaintext.

## Mode matrix

| Mode | Time | What runs |
|---|---|---|
| `quick` | ~20s | `cargo test -p fastc` only |
| `ci` | ~30s | `cargo fmt --check` + `cargo clippy` + `cargo test --workspace` |
| `full` *(default)* | ~2 min | `ci` + doc-tests + release build + examples smoke |

### When to use each

- **`quick`** — while iterating on a single change. You want the
  tightest feedback loop, so the harness only touches the `fastc`
  crate and skips format / clippy / examples.
- **`ci`** — right before you push. Same checks the GitHub Actions
  workflow runs, so if `ci` passes locally, CI will pass too.
- **`full`** *(default)* — before opening a PR, or any time you
  changed the lower pass, runtime, or examples. This also smoke-tests
  a representative subset of `examples/*.fc` end-to-end through
  `fastc compile` + `cc` so emit / runtime regressions surface
  before review.

The harness prints a banner per step, the exact `cargo` command it
runs, and a per-step duration — so when something fails you can
re-run the offending command directly.

## scripts/check.sh — pre-PR alias

```bash
bash scripts/check.sh
```

`check.sh` is a one-line alias for `test.sh ci`. It exists so you
can wire a single command into a pre-push hook or muscle memory
without thinking about modes. Same exit codes, same output, same
duration as `test.sh ci`.

If you'd like it to run automatically before every push, drop this
into `.git/hooks/pre-push` and make it executable:

```bash
#!/usr/bin/env bash
bash scripts/check.sh
```

## scripts/bench.sh — benchmark suite

```bash
bash scripts/bench.sh
```

`bench.sh` wraps `benchmarks/run_all.sh` and runs the
cross-language benchmark suite (fastC vs. C, Rust, Zig, Go).
Expect ~5 minutes on a modern laptop. It produces a `results.csv`
and prints a per-benchmark summary table.

This is *not* part of `test.sh` — benchmarks are slow and noisy
under CPU contention, so they're a separate script you opt into
when you're measuring perf changes.

See [`fastc bench`](../cli/bench.md) for the per-project benchmark
runner exposed through the CLI.

## Writing tests

Tests live where Cargo expects them — the harness doesn't need
updating when you add a new file, because it delegates to
`cargo test` which discovers everything automatically.

- **Unit tests** — `#[cfg(test)] mod tests` inside
  `crates/fastc/src/<module>.rs`. Use these for anything that
  exercises a single function or a small set of helpers.
- **Integration tests** — a new `.rs` file under
  `crates/fastc/tests/`. Each file is a separate crate, so it
  only sees the public API of `fastc`. Use these for full-pipeline
  tests that drive `compile()` / `check()` / `parse()` end-to-end.
- **Doc tests** — `///` triple-slash comments with `# Examples`
  blocks containing runnable Rust. Doc tests run under
  `test.sh full` (and on CI).

If you're testing fastC source code rather than Rust internals, the
common pattern is:

```rust
use fastc::compile;

#[test]
fn my_feature_works() {
    let src = r#"
        fn main() -> i32 {
            return 42;
        }
    "#;
    let c = compile(src, "my_test.fc").expect("should compile");
    assert!(c.contains("return 42"));
}
```

The integration tests under `crates/fastc/tests/` are the best
existing reference — they cover modules, the CLI, source maps,
discharge, supply chain, and more.

## Inline test { } blocks

fastC 1.0 ships an inline `test { }` block syntax for assertions
that live alongside the code they exercise:

```c
test {
    fn it_adds() -> i32 {
        return ((2 + 2) - 4);  // 0 = pass
    }
    fn it_doubles() -> i32 {
        return 0;
    }
}

fn main() -> i32 {
    return 0;
}
```

How it works:

- Every `fn` inside a `test { }` block is implicitly marked
  `@test`.
- In a normal build, the v1.0 driver strips `@test` fns from the
  AST before lowering — they don't appear in the emitted C, so
  there's zero runtime cost to leaving them in your source.
- `fastc compile --test` (a v1.x follow-up) will gate them *in*
  and generate a runner `main` that invokes each test fn and
  reports pass / fail.

The contextual keyword only hijacks `test` immediately followed by
an open brace, so a user fn named `fn test()` continues to parse
exactly as before.

When to use it:

- Quick assertions inside a single source file you're iterating on.
- One-file demos that ship with their own self-tests.
- *Not* yet a full test runner — for a real test suite, write a
  Rust integration test in `crates/fastc/tests/` against the
  `compile()` API. The inline block is convenience syntax, not a
  replacement for the harness.

## What CI runs

The GitHub Actions workflow at `.github/workflows/ci.yml` invokes
`scripts/test.sh ci` on every push and pull request. That means
your local `bash scripts/test.sh ci` (or `bash scripts/check.sh`)
gives you the exact same pass/fail signal CI will — no surprises
between your machine and the green checkmark.

If a step is *optional* in the harness (currently `cargo clippy`
and doc-tests), CI treats it the same way: a non-zero exit is
flagged but does not fail the build. The summary at the end of
each run shows skipped vs. failed counts separately.

## Test count

As of v1.0, the workspace has **340+ tests passing** across unit,
integration, doc, and end-to-end smoke layers. `test.sh quick`
runs the bulk of them in ~20 seconds; `test.sh full` covers the
rest plus the examples smoke test in ~2 minutes.

If you add a substantial feature, please add at least one
integration test under `crates/fastc/tests/` that exercises it
through the public `compile()` / `check()` API — that way the
harness will catch regressions automatically forever.

## Cross-links

- CLI: [`fastc bench`](../cli/bench.md) — per-project benchmark
  runner.
- CLI: [`fastc check`](../cli/index.md) — fast type-check without
  emitting C.
- [Why benchmarks](../why/benchmarks.md) — the rationale behind the
  numbers in `bench.sh`.
- [`scripts/README.md`](https://github.com/Skelf-Research/fastc/blob/main/scripts/README.md)
  in the source tree — the canonical usage matrix this page
  mirrors.
