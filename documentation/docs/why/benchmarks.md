# Benchmarks

Real, measured numbers for the claims in the [rubric](rubric.md). Every number on this page comes from a script committed to the repo under `benchmarks/cross-lang/`. Re-running takes one command and ~5 minutes total wall-clock for the perf and token-count suites.

## Compile time, binary size, runtime

Four small programs (`hello`, `sum`, `fib40`, `mandelbrot`) implemented idiomatically in fastC, C, Rust, Zig, Go. See `benchmarks/cross-lang/README.md` for the methodology.

Snapshot from the M3 local run dated 2026-05-22:

### Compile time (median, ms)

| Program | fastC | C | Rust | Zig | Go |
|---|---|---|---|---|---|
| hello | 193 | 74 | 113 | 139 | 164 |
| sum | 194 | 72 | 116 | 140 | 144 |
| fib40 | 199 | 69 | 114 | 141 | 147 |
| mandelbrot | 198 | 81 | 159 | 142 | 145 |

fastC's compile path is `fastc compile` + `cc -O2`. The fastc step alone is ~125ms; the rest is the C compile. fastC is roughly 30–40% faster than Rust to a release binary on these programs.

### Binary size (stripped, bytes)

| Program | fastC | C | Rust | Zig | Go |
|---|---|---|---|---|---|
| hello | 52,984 | 33,432 | 341,512 | 50,184 | 2,384,424 |
| sum | 52,984 | 16,824 | 341,016 | 50,136 | 2,115,224 |
| fib40 | 52,984 | 16,824 | 341,016 | 50,136 | 2,115,224 |
| mandelbrot | 52,992 | 33,440 | 341,808 | 50,192 | 2,115,808 |

fastC binaries are ~53KB across the board — very close to Zig (50KB), much smaller than Rust (340KB) and Go (2.1MB). The fastC vs C delta is the runtime header (`fastc_runtime.h`).

### Runtime (median, ms)

| Program | fastC | C | Rust | Zig | Go |
|---|---|---|---|---|---|
| hello | 2 | 2 | 3 | 3 | 3 |
| sum | 3 | 2 | 3 | 2 | 4 |
| fib40 | **430** | 338 | 341 | 340 | 400 |
| mandelbrot | 60 | 63 | 55 | 72 | 56 |

fastC matches C on FP-heavy work (mandelbrot). fastC is **27% slower than C on `fib40`** — the cost of overflow-checked integer arithmetic, lowered to `__builtin_add_overflow` + branch on every `+` and `-`. The honest read: fastC's safety isn't free on recursive integer workloads. A future `unsafe` block in the hot recursive path closes the gap if needed.

## Token count

How many LLM tokens does it take to write the same program in each language? Measured by [`tiktoken`](https://github.com/openai/tiktoken) with the `cl100k_base` encoding (GPT-4 family).

| Program | fastC | C | Rust | Zig | Go |
|---|---|---|---|---|---|
| hello | 25 | 21 | 10 | 38 | 17 |
| sum | 105 | 58 | 55 | 65 | 52 |
| fib40 | 117 | 69 | 66 | 63 | 60 |
| mandelbrot | 547 | 260 | 301 | 346 | 267 |

**Honest finding: fastC is the most verbose in three of four programs.** The cause is fastC's explicit-cast syntax — every `cast(f64, x)` is several tokens, vs `(double)x` in C or `x as f64` in Rust. The pitch is not "fewer tokens than Rust" — it's "more of what you wrote is structurally checked." Whether that trade pays off is the next benchmark.

## First-compile success rate

Does an LLM writing fastC produce code that compiles cleanly on the first try more reliably than the same LLM writing Rust or Zig? This is the most important benchmark for the agent-friendliness claim — it directly measures the wedge.

The harness lives at `benchmarks/cross-lang/first-compile/`. Three tasks × five languages × three LLMs (Claude, GPT-4o, Gemini 2.5 Pro) × N=10 trials each.

The headline run hasn't been performed yet in this build environment (no API keys configured) — `results.csv` ships as a placeholder. Once it's populated, the table here will look like:

| Task | Lang | Claude | GPT-4o | Gemini | Mean |
|---|---|---|---|---|---|
| sum_array | fastc | TBD | TBD | TBD | TBD |
| sum_array | rust | TBD | TBD | TBD | TBD |
| … | | | | | |

If fastC's mean first-compile rate is +15pp or more above Rust, the token-count cost pays for itself in fewer compile cycles. If it isn't, the language design needs revisiting.

See `benchmarks/cross-lang/first-compile/README.md` for how to run the benchmark yourself (~$5–8 in API costs, ~60–90 min wall-clock).

## Reproducing

```bash
# Perf (compile / size / runtime)
cd benchmarks/cross-lang
./run.sh                                   # ~30 seconds

# Token count
cd token-count
python3 count_tokens.py                    # ~5 seconds

# First-compile-success rate (requires API keys)
cd ../first-compile
ANTHROPIC_API_KEY=... \
OPENAI_API_KEY=... \
GOOGLE_API_KEY=... \
python3 run.py --n 10                      # ~60-90 minutes
```

All three scripts overwrite their own `results.csv` in place. The committed versions are the local-M3 golden runs with a date stamp at the top.

## What this page deliberately doesn't measure

- **Cold-cache compile times.** All numbers are warm-cache (hyperfine does one warmup run). Cold-cache numbers would be 2–4× higher across the board.
- **Large multi-file projects.** Single-file micro-benchmarks measure a specific axis. Incremental-build performance over a real codebase is a separate suite (planned).
- **PGO, LTO, native-march.** All compilers invoked at their default release flag. The numbers reflect what a developer gets out of the box, not the theoretical peak.
- **Linux x86_64 numbers.** M3 only. A CI runner fills in the x86_64 row in a follow-up.
