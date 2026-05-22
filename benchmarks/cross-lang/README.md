# Cross-language benchmarks

Honest, reproducible compile-time / binary-size / runtime numbers
for fastC vs C, Rust, Zig, and Go on four small programs.

## What's measured

| Program | Algorithm |
|---|---|
| `hello` | `puts("Hello\n")` |
| `sum` | Sum 1..1_000_000 (i64), return mod 256 |
| `fib40` | Recursive `fib(40)`, return mod 256 |
| `mandelbrot` | 800×800 mandelbrot to stdout |

Three axes per program:

- **Compile time** — median of 3 runs via `hyperfine`, source to stripped binary.
  - fastC = `fastc compile` + `cc -O2`.
  - C = `gcc -O2`.
  - Rust = `rustc -O`.
  - Zig = `zig build-exe -O ReleaseFast -lc` (libc linked so `puts` etc resolve).
  - Go = `go build`.
- **Binary size** — `strip`-ed output, bytes via `stat`.
- **Runtime** — median of 5 runs via `hyperfine --ignore-failure` (programs return computed values as exit codes, so non-zero exit is correct).

## Reproducing

```bash
# One-time setup
brew install hyperfine
cargo build --release -p fastc

# Run the suite (~30 seconds wall-clock)
./run.sh

# Output: results.csv overwritten in this directory
cat results.csv
```

The script tolerates a missing Go install (sets `N/A` in the Go row); for other languages, missing toolchains will cause hyperfine to fail loudly. Pass `GO_BIN=...` to override Go path; default auto-discovers brew installs.

## Headline numbers (current local M3 run, 2026-05-22)

Picking representative rows from `results.csv`:

| Program | Lang | Compile (ms) | Strip (bytes) | Size vs fastC | Runtime (ms) |
|---|---|---|---|---|---|
| hello | fastC | 215 | 53,080 | 1.00× | 1 |
| hello | C | 76 | 33,432 | 0.63× | 2 |
| hello | Rust | 118 | 341,512 | **6.43×** | 2 |
| hello | Zig | 149 | 50,184 | 0.95× | 2 |
| hello | Go | 164 | 2,384,424 | **44.92×** | 3 |
| mandelbrot | fastC | 218 | 53,088 | 1.00× | 63 |
| mandelbrot | C | 79 | 33,440 | 0.63× | 62 |
| mandelbrot | Rust | 163 | 341,808 | **6.44×** | 56 |
| mandelbrot | Zig | 156 | 50,192 | 0.95× | 77 |
| mandelbrot | Go | 155 | 2,115,808 | **39.86×** | 57 |

fastC's binary size sits in the C / Zig class — within 1 KB of Zig, ~50 KB above C (the cost of the safety runtime). Rust is 6.4× larger; Go is 40× larger. For container cold-start, embedded targets, and audit-by-disassembly workflows the order-of-magnitude advantage compounds.

The full set lives in `results.csv` with a date stamp and host info at the top.

## Known caveats

1. **fastC's compile time includes the `fastc → C` step plus `cc -O2`.** The C step alone is ~70ms; fastC's own overhead is ~120–130ms on these programs. We measure end-to-end wall-clock because that's what a user actually waits for.
2. **fib40 is 25–30% slower in fastC than in C.** The overhead comes from fastC's overflow checks on every signed `+`/`-` (lowered to `__builtin_add_overflow`/`__builtin_sub_overflow` + branch). Mandelbrot doesn't show this because FP arithmetic isn't overflow-checked. A future `unsafe` block in the hot recursive path would close the gap.
3. **fastC's mandelbrot uses a sentinel variable instead of `break`.** The fastC v1 lower pass silently drops `break` inside `while` loops (the catch-all `_ => vec![]` arm in `lower_stmt` swallows it). Filed as a compiler bug; the workaround in `mandelbrot/fc/main.fc` is structurally equivalent and produces identical output to the other four languages (`sha256` matches across all five).
4. **Linux x86_64 numbers** aren't included yet. The M3 row is the local-run golden; a CI runner will fill in the second row in a follow-up.
5. **Runtime variance** is ±20% on these small programs. Hyperfine reports the median, but anything under ~10ms is dominated by process-startup noise.

## What this benchmark doesn't measure

- **Optimization passes.** All compilers invoked at default-release. No `-flto`, no `-march=native`, no PGO. The numbers reflect what a developer gets out of the box.
- **Cold-cache compile.** Hyperfine does one warmup run. Cold-cache numbers would be 2–4× the warm numbers for every language, mostly equally.
- **Build of a large project.** These are single-file programs. fastC's incremental compilation story (Salsa-based) is the right benchmark for multi-file projects; that's a separate suite.
- **Runtime correctness.** The benchmark assumes the programs work; we verified manually that mandelbrot output bytes match across all five languages (`sha256 = 56eab454…`). `sum` and `fib40` exit with their computed values (32 and 203) on every language.
