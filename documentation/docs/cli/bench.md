# Bench Command

The `bench` command runs the compile-time budget gate. fastC commits to
inner-loop compile speed as a contract: every PR runs `fastc bench` in CI,
and if any of the declared targets regress past their threshold, the build
fails. The budget file is the contract; this command is the enforcer.

## Usage

```bash
fastc bench [OPTIONS]
```

## Options

| Option | Description |
|--------|-------------|
| `--budget <PATH>` | Path to the budget TOML (default: auto-discovered by walking up from the current directory) |
| `--fail-on-regression` | Exit 1 if any budget is over its target. CI-friendly. |
| `--only <NAME>` | Only run the named benchmark (matches a key under `[budgets.*]`) |
| `-h, --help` | Print help |

The JSON report path and markdown report path are not flags — they are
declared inside `compile-time-budget.toml` under `[reporting]`. The
human-readable markdown table always streams to stderr.

## The Budget File

`bench` reads `compile-time-budget.toml` at the project root. A typical
file looks like this:

```toml
[budgets.clean_examples]
description = "Clean build of curated tutorial examples"
target_ms = 2000
regression_threshold = 0.20
inputs = "examples/tutorials/*.fc"
mode = "compile"

[budgets.clean_fastc_crate]
description = "Clean build of crates/fastc itself (cargo build --release)"
target_ms = 10000
regression_threshold = 0.20
mode = "cargo_build_release"

[budgets.incremental_edit]
description = "Re-check a single file after a no-op edit (Salsa cache warm)"
target_ms = 200
regression_threshold = 0.20
inputs = "examples/advanced/algorithms.fc"
mode = "check_warm"

[budgets.single_file_check]
description = "Cold-cache `fastc check` on a representative file"
target_ms = 500
regression_threshold = 0.30
inputs = "examples/advanced/algorithms.fc"
mode = "check_cold"

[measurement]
runs_per_benchmark = 5
warmup_runs = 2
report_metric = "min"

[reporting]
emit_json = ".fastc/timing/budget.json"
emit_markdown = ".fastc/timing/budget.md"
```

### The Four Default Targets

| Budget key | What it measures |
|------------|------------------|
| `clean_examples` | Wall-clock to compile every `examples/*.fc` from scratch |
| `clean_fastc_crate` | `cargo build --release -p fastc` from a clean cargo cache |
| `incremental_edit` | Re-check a single file after a no-op edit; the Salsa cache hit |
| `single_file_check` | `fastc check <file>` on a representative example, cold cache |

`mode` selects the harness behaviour: `compile`, `compile_dev`,
`check_cold`, `check_warm`, or `cargo_build_release`. `inputs` is a glob
resolved relative to the project root.

### Measurement Knobs

| Key | Default | Meaning |
|-----|---------|---------|
| `runs_per_benchmark` | 5 | Total runs after warmup |
| `warmup_runs` | 2 | Untimed runs before measurement starts |
| `report_metric` | `"min"` | Aggregation: `"min"` filters one-time noise, `"median"` is more robust on noisy CI |

### Reporting Knobs

| Key | Default | Meaning |
|-----|---------|---------|
| `emit_json` | none | Where to write the JSON artifact (relative to project root) |
| `emit_markdown` | none | Where to write the markdown summary |

## Sample Output

The markdown report streams to stderr at the end of every run:

```
# fastC compile-time budget

fastc 1.0.0 | host: aarch64-apple-darwin

| Benchmark | Target | Measured | Δ | Status |
|-----------|-------:|---------:|--:|:------:|
| clean_examples | 2000ms | 1684ms | -15.8% | ✓ |
| clean_fastc_crate | 10000ms | 8921ms | -10.8% | ✓ |
| incremental_edit | 200ms | 147ms | -26.5% | ✓ |
| single_file_check | 500ms | 412ms | -17.6% | ✓ |

Overall: Pass
```

The JSON artifact has the same shape, suitable for downstream tooling:

```json
{
  "fastc_version": "1.0.0",
  "host": "aarch64-apple-darwin",
  "results": [
    {
      "name": "clean_examples",
      "target_ms": 2000,
      "measured_ms": 1684,
      "status": "Pass"
    }
  ],
  "overall_status": "Pass"
}
```

## Status Classification

For each benchmark, `bench` compares the measured time to `target_ms`:

| Status | Condition |
|--------|-----------|
| `Pass` | `measured <= target` |
| `Warn` | `target < measured <= target * (1 + regression_threshold)` |
| `Fail` | `measured > target * (1 + regression_threshold)` |
| `Skip` | The benchmark could not run (missing inputs, unsupported mode) |

The overall status is the worst of all individual statuses.

## Examples

### Local Run

```bash
fastc bench
```

Walks up from the current directory to find `compile-time-budget.toml`,
runs every benchmark, prints the markdown table to stderr, and writes the
JSON + markdown artifacts to the paths declared under `[reporting]`.

### CI Gate

```bash
fastc bench --fail-on-regression
```

Same as above, but exits 1 if `overall_status` is `Fail`. Drop this into
the CI workflow as the budget gate.

### Single Benchmark

```bash
fastc bench --only incremental_edit
```

Useful when iterating on a specific hot path — e.g. tuning the Salsa
warm-cache numbers without paying for the full clean-build sweep.

### Explicit Budget Path

```bash
fastc bench --budget ./ci/budget.toml --fail-on-regression
```

For projects that store the budget file outside the auto-discovery path,
or that maintain multiple budget profiles (CI vs. local).

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Bench completed; with `--fail-on-regression`, overall status is `Pass` or `Warn` |
| 1 | With `--fail-on-regression`, overall status is `Fail` |

Without `--fail-on-regression`, `bench` exits 0 even on regressions —
the human reads the table and decides.

## See Also

- [Testing & CI](../getting-started/testing.md) — wiring `bench` into the
  CI gate
- [Benchmarks](../why/benchmarks.md) — the methodology behind the
  numbers and the rationale for the four targets
- [cert-report](cert-report.md) — the other CI gate, for Power-of-10
  compliance
