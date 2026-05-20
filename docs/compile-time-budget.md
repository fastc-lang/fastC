# Compile-Time Budget

This document specifies fastC's compile-time discipline: measured targets, the infrastructure to enforce them, and the structural choices that make them achievable.

The budget lands in **stage 0.8** of the [roadmap](roadmap.md), before stdlib. The placement is deliberate: every subsequent stage must land under measurement, so no feature can bloat compile times without anyone noticing until it is too late to fix.

## Targets

Hard targets, measured in CI on every push:

| Metric | Target | Measured on |
|--------|:------:|-------------|
| Clean build of `examples/` (all 20 tutorial + advanced programs) | **< 2.0s** | `--dev` mode, tcc backend |
| Clean build of `crates/fastc/` itself (`cargo build`) | **< 10.0s** | release mode |
| Incremental edit (single function changed, rebuild + check) | **< 200ms** | `--dev` mode, Salsa cache warm |
| Clean build of an HTTP+TLS server (the launch benchmark) | **< 1.5s** | `--dev` mode |
| Single-file `fastc check` (cold cache) | **< 500ms** | for any `.fc` file under 1000 lines |

These are not aspirations. They are CI gates: any PR that regresses any target by more than 20% fails the build. The regression threshold is high enough to absorb genuine compiler improvements that add cost in one pass while saving more elsewhere, but low enough that a no-op O(n²) pass added to the pipeline is caught immediately.

## Why these numbers

The targets are calibrated against existing systems languages on equivalent workloads:

| Language | HTTP+TLS server clean build | Why |
|----------|:---------------------------:|-----|
| Go | ~1.8s | Stackful coroutines, no monomorphization, simple type system |
| Zig | ~3.4s | comptime + LLVM backend |
| Rust | ~14.7s | Monomorphization fan-out, async machinery, proc macros |
| C (gcc, hand-tuned) | ~0.6s | Baseline; what we are asking the C compiler to do |
| **fastC (target)** | **< 1.5s** | tcc dev backend, no proc macros, no async machinery, small surface |

We are aiming to beat Go on clean builds in `--dev` mode, and to lose to Rust by an order of magnitude. We will lose to hand-tuned C, deliberately: fastC's transpilation adds a real cost (the lex/parse/typecheck/lower/emit/cc pipeline runs end-to-end) which is the price of the safety and the annotation system.

The `< 200ms` incremental target is the agent-iteration latency floor. An agent that runs `fastc check` 20 times per minute should not be blocked on the compiler. Salsa caching makes this achievable on edits that change one function: only that function's passes re-run, plus the C output for that function only.

## The structural advantages

fastC sidesteps the three things that made Rust slow:

1. **No monomorphization fan-out at scale.** Generics monomorphize (stage 0.9), but the language has no `dyn Trait`, no associated types, no GATs — the type-system surface is small enough that monomorphization cost grows linearly with code, not super-linearly with abstraction layers.
2. **No proc macros.** Compile-time code execution is the single biggest contributor to Rust compile times beyond LLVM. fastC has none.
3. **C backend.** gcc and clang already optimize C aggressively. We do not own an LLVM backend; we hand off to a battle-tested C compiler. tcc handles `--dev` mode at ~100MB/s for development inner loops.

Plus three structural choices fastC makes specifically:

1. **Salsa-style query system from day one.** Every compiler pass (parse, resolve, typecheck, lower, emit, annotation check, capability check, contract discharge) is a pure function of its inputs, cached by input hash. An incremental edit re-runs only the queries whose inputs changed.
2. **Module-level parallelism.** The module-graph build pass (from stage 1.3) resolves the DAG up front; module compilation dispatches to a work-stealing pool. No within-module parallelism — gains are small and implementation is brittle.
3. **tcc dev backend.** `fastc build --dev` uses TinyCC, which compiles C at ~100MB/s — essentially free. `fastc build --release` uses gcc/clang for optimization. Agents and developers iterate on `--dev`; production builds use `--release`.

## The infrastructure

### `compile-time-budget.toml`

Lives at the repo root. Format:

```toml
[budgets]
clean_examples = { target_ms = 2000, regression_threshold = 0.20 }
clean_fastc_crate = { target_ms = 10000, regression_threshold = 0.20 }
incremental_edit = { target_ms = 200, regression_threshold = 0.20 }
http_tls_server = { target_ms = 1500, regression_threshold = 0.20 }
single_file_check = { target_ms = 500, regression_threshold = 0.30 }

[measurement]
runs_per_benchmark = 5
warmup_runs = 2
report_metric = "min"  # min across runs, to filter noise

[reporting]
emit_json = ".fastc/timing/budget.json"
emit_markdown = ".fastc/timing/budget.md"
```

### CI gate

Every PR runs the budget check. The output is a markdown table posted as a check comment:

```
| Metric                    | Target | Current | Δ from prev | Status |
|---------------------------|-------:|--------:|------------:|:------:|
| Clean build of examples/  | 2000ms | 1842ms  |       +3.2% |   ✓    |
| Clean build of fastc crate| 10000ms| 9120ms  |       -1.1% |   ✓    |
| Incremental edit          |  200ms | 167ms   |       +0.8% |   ✓    |
| HTTP+TLS server           | 1500ms | 1240ms  |       -2.4% |   ✓    |
| Single-file check         |  500ms | 412ms   |       +0.4% |   ✓    |
```

Any row exceeding `target_ms × (1 + regression_threshold)` versus the previous green build fails CI. The PR author either fixes the regression or justifies it explicitly (e.g., "added contract discharge, +18% to single-file check, within budget").

### Per-pass timing

`fastc build --timing` emits per-pass timing into `.fastc/timing/pass-timing.json`:

```json
{
  "input": "examples/05_http_server.fc",
  "total_ms": 1240,
  "passes": [
    {"pass": "lex",                  "ms": 12,  "cache": "miss"},
    {"pass": "parse",                "ms": 38,  "cache": "miss"},
    {"pass": "module_graph",         "ms": 5,   "cache": "hit"},
    {"pass": "resolve",              "ms": 41,  "cache": "miss"},
    {"pass": "typecheck",            "ms": 67,  "cache": "miss"},
    {"pass": "annotation_check",     "ms": 24,  "cache": "miss"},
    {"pass": "capability_check",     "ms": 18,  "cache": "miss"},
    {"pass": "contract_discharge",   "ms": 31,  "cache": "miss"},
    {"pass": "panic_analysis",       "ms": 9,   "cache": "miss"},
    {"pass": "lower",                "ms": 52,  "cache": "miss"},
    {"pass": "emit",                 "ms": 28,  "cache": "miss"},
    {"pass": "cc (tcc)",             "ms": 915, "cache": "miss"}
  ],
  "salsa_cache_hits": 3,
  "salsa_cache_misses": 11
}
```

This is the diagnostic surface for tuning the compiler. It is also what `fastc-mcp` (stage 1.6) serves to coding agents when they ask "why is my build slow?"

## The Salsa skeleton (stage 0.8)

The Salsa framework — extracted from rust-analyzer (originally from rustc) — is the right model. Read [the Salsa docs](https://github.com/salsa-rs/salsa) and [the rustc query system docs](https://rustc-dev-guide.rust-lang.org/query.html) before implementing this. There is a decade of hard-won wisdom about query granularity and cycle handling.

Key choices for fastC:

- **Query granularity: per function and per module.** A "function AST" query and a "function typecheck" query are separate; a typecheck edit does not invalidate the parse cache.
- **Hash-based invalidation.** Each query's inputs are SHA-256-hashed. The hash chain propagates: an edit to a function changes its hash; queries depending on that function's hash re-run; queries that did not depend on it are unaffected.
- **No cycles in v1.** Salsa supports cycle detection; fastC's compiler pipeline is acyclic by design. Cycle detection is a v2 problem when (and if) we add mutually recursive type definitions.
- **Persistent cache.** Salsa state lives in `.fastc/cache/salsa/`. A cold start reads the cache and resumes from prior queries. Cache invalidation on `fastc` version change is automatic (the version is in the cache key).

The skeleton is roughly 2000–3000 lines of Rust if we follow Salsa's pattern closely.

## SMT discharge cache (stage 2.1)

When SMT contract discharge ships in stage 2.1, the SMT calls are budgeted at 500ms per obligation. Without caching, a 5000-line project with 400 contract obligations could spend 200 seconds in Z3 on a clean build. With caching:

- Each SMT formula is hashed (SHA-256 of the normalized SMT-LIB text).
- Discharge results (`unsat` / `sat` / `unknown` + counter-example if `sat`) are stored in `.fastc/cache/contract_discharge/<hash>.json`.
- Cache hits are O(filesystem). A re-build that touches no contracts is instant on the discharge pass.

CI runs full SMT discharge with a cold cache; the developer inner loop uses `--no-prove` to skip SMT entirely. After the first CI run, the cache is populated, and subsequent runs are dominated by the few obligations that actually changed.

## The tcc dev backend

TinyCC is a fast (~100MB/s), small (~200KB binary), reasonably-conformant C99/C11 compiler. fastC uses tcc as the codegen backend for `--dev` builds:

- `fastc build --dev` pipes the emitted C into tcc and produces a runnable binary in single-digit milliseconds for small programs.
- No optimization. The output is a debug binary suitable for tests, scratch programs, and the agent inner loop.
- `--release` uses gcc or clang (configured via `fastc.toml` or env var). The output is optimized, debuggable, and what ships to production.

This is fastC's secret weapon: emit-C languages can use tcc as a fast prototype backend in a way self-hosted compilers (Zig) and LLVM-bound compilers (Rust) cannot. The user experience approaches Python's "run this script" latency for small fastC programs.

Caveats:

- tcc is C99/C11-conformant but not bug-free. Differences against gcc/clang have been seen in edge cases. fastC's emitted C is designed to avoid known tcc bugs (no nested function declarations, no GNU-isms).
- tcc does no warnings. fastC's own diagnostic surface fully replaces what `-Wall` would do — type errors and safety violations are caught before the C is even emitted.
- tcc has no LTO, no PGO, no auto-vectorization. `--release` mode handles all of that via clang/gcc.

## Anti-patterns

The budget exists to prevent these specific failure modes, which killed every "safer C" predecessor:

1. **"We'll optimize the compiler later."** Compile-time regressions compound. A 5ms regression per file × 1000 files = 5s. The budget catches it on day 1.
2. **"This feature only adds 50ms."** Five features that each add 50ms add 250ms — enough to blow the incremental-edit target. The budget catches it cumulatively.
3. **"The big one-time fix will recover the budget."** This rarely happens. Once the budget is violated, the slack disappears, and every subsequent feature is fighting upstream.
4. **"Our users have fast machines."** The budget is set against a known reference machine spec (see below). CI measures on that spec. Real users have slower machines and worse cache states than CI.

## Reference machine spec

CI runs on:

- AWS `c7a.large` (2 vCPU, 4 GiB RAM, AMD Zen 4).
- Ubuntu 24.04 LTS.
- gcc 14, clang 18, tcc 0.9.27 (built from upstream HEAD).
- All targets pinned by hash.

User-side compile times will vary with machine, but the budget targets are designed to hold comfortably on a modest developer laptop (M1 / Ryzen 5 / equivalent) with 3–5× headroom.

## What is not covered by the budget

- **SMT discharge time on cold cache.** CI runs SMT discharge separately and reports its time, but does not gate on it — Z3 performance is too variable to gate. The cache makes subsequent runs fast; the cold run is an acceptable CI cost.
- **Test suite execution.** `cargo test` is timed separately and not part of the compile-time budget.
- **Documentation generation.** `fastc doc` (when it ships) is excluded.

## Open questions

- **Cross-platform CI budgets.** Currently the reference is Linux x86_64. Should we set separate targets for ARM macOS and Windows x64? Current lean: yes, with relaxed targets (1.5×) until Salsa has tuned for each platform.
- **First-build overhead.** A clean checkout has no Salsa cache. Should the first-build target be separate from subsequent builds? Current answer: no, the targets are for cold-cache clean builds.
- **What to do on regression.** Hard fail (current) or warn + require justification in PR description? Current lean: hard fail for the main metrics, warn for the long-tail metrics. Revisit after 0.8 ships and we see real regression patterns.
