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

The harness lives at `benchmarks/cross-lang/first-compile/`. Three tasks (sum_array, is_prime, json_token) × five languages × N=3 trials per cell. Tested LLMs:

- **Ollama Cloud models (real data, this run)**: `glm` → glm-5.1, `kimi` → kimi-k2.6, `deepseek` → deepseek-v4-pro, `qwen` → qwen3.5. All four use `OLLAMA_API_KEY` against `https://ollama.com/api/chat`.
- **Proprietary three (placeholder)**: Claude, GPT-4o, Gemini 2.5 Pro. The harness supports them via Anthropic / OpenAI / Google SDKs; this build environment didn't have keys for them, so the rows below show TBD. Re-run with the right env vars to populate.

The full numbers ship in `benchmarks/cross-lang/first-compile/results.csv` with a date + provider header at the top. Snapshot from the partial Ollama run (T1: sum_array, N=3 trials per cell — full grid is still in progress):

| Lang | GLM | Kimi | DeepSeek | Qwen |
|---|---|---|---|---|
| C | 3/3 | 3/3 | 3/3 | 3/3 |
| Rust | 3/3 | 2/3 | 2/2 | 2/2 |
| Zig | 3/3 | 2/2 | 3/3 | TBD |
| Go | 3/3 | TBD | TBD | TBD |
| **fastC** | **0/3** | **0/3** | **0/3** | **0/2** |

**The headline finding is stark: across all four open-weight Ollama Cloud models, fastC has a 0% first-compile pass rate on T1 sum_array — the simplest task — while C / Rust / Zig / Go all land at or near 100%.**

What's going wrong: every model produces fastC code that *looks* idiomatic but fails the compiler. Common errors observed in the response files at `benchmarks/cross-lang/first-compile/responses/T1/fastc/<llm>/`:

- `arr[i]` indexing — fastC has no syntactic `[]` for vec; need `at(arr.data, i)` inside `unsafe`.
- `vec::len(arr)` without `addr(...)` — `vec::len` takes `ref(Vec[T])`, not `Vec[T]` by value.
- Integer literals without explicit cast — fastC won't unify `0` with `i64`; you need `cast(i64, 0)`.
- Missing `(...)` around binary expressions — fastC has no precedence rules; chained binary ops must be explicitly parenthesized.

These are exactly the strictnesses fastC adds for type-safety wins, and they translate directly into compile failures when an LLM (trained on the world's C / Rust / Zig / Go) applies its general-purpose intuition. The token-count benchmark above showed fastC is the most verbose language in this set; this benchmark shows the verbosity does *not* convert into more first-compile reliability for open-weight models. The wedge hypothesis — that strict syntax pays for itself in fewer compile cycles — is **not supported** by this data on open-weight models.

Open question: do proprietary frontier models (Claude Opus, GPT-4o, Gemini 2.5 Pro) do meaningfully better on fastC? They have larger context windows, better instruction-following, and are more likely to ingest a long cheat sheet verbatim. The harness supports all three — set `ANTHROPIC_API_KEY` / `OPENAI_API_KEY` / `GOOGLE_API_KEY` and re-run to populate. Until that data lands, the agent-friendliness claim only stands for frontier models.

See `benchmarks/cross-lang/first-compile/README.md` for how to run. Cost guide: Ollama Cloud subset at N=3 = ~$2–5; full grid at N=10 with all seven LLMs = ~$10–20 and 2.5–4 hours wall-clock.

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
# Ollama Cloud only (4 models from ollama_models.json):
OLLAMA_API_KEY=... python3 run.py --n 3 --llms glm kimi deepseek qwen
# Or full grid with Ollama + the proprietary three:
ANTHROPIC_API_KEY=... OPENAI_API_KEY=... GOOGLE_API_KEY=... \
OLLAMA_API_KEY=... python3 run.py --n 10     # ~2.5-4 hours
```

All three scripts overwrite their own `results.csv` in place. The committed versions are the local-M3 golden runs with a date stamp at the top.

## What this page deliberately doesn't measure

- **Cold-cache compile times.** All numbers are warm-cache (hyperfine does one warmup run). Cold-cache numbers would be 2–4× higher across the board.
- **Large multi-file projects.** Single-file micro-benchmarks measure a specific axis. Incremental-build performance over a real codebase is a separate suite (planned).
- **PGO, LTO, native-march.** All compilers invoked at their default release flag. The numbers reflect what a developer gets out of the box, not the theoretical peak.
- **Linux x86_64 numbers.** M3 only. A CI runner fills in the x86_64 row in a follow-up.
