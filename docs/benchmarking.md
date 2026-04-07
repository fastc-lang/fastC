# Benchmarking Methodology

This document describes FastC's benchmarking infrastructure: what we measure, how we measure it, and how we report results. The goal is rigorous, reproducible, and honest performance comparison against C, Zig, and Rust.

Benchmarking is planned for the [1.1 milestone](roadmap.md).

## Benchmark Suite Selection

### CLBG Programs

We use six programs from the [Computer Language Benchmarks Game](https://benchmarksgame-team.pages.debian.net/benchmarksgame/) (CLBG). These are well-understood, compute-intensive workloads that stress different aspects of a language runtime:

| Program | Primary stress | Why it matters for FastC |
|---------|---------------|------------------------|
| **n-body** | Floating-point arithmetic, struct access | Measures overhead of safety checks on numeric ops |
| **binary-trees** | Allocation/deallocation, tree traversal | Tests `own(T)` and `Drop` performance |
| **spectral-norm** | Dense linear algebra, loops | Tests loop bounds-check elimination |
| **mandelbrot** | Bit manipulation, tight loops | Tests raw compute parity with C |
| **fannkuch-redux** | Array permutation, integer ops | Tests slice bounds-check overhead |
| **fasta** | String/byte buffer output, I/O | Tests I/O and string standard library performance |

Each program is implemented idiomatically in each language — no artificial handicaps, no tricks to favor one language over another.

### Micro-Benchmarks

Micro-benchmarks isolate specific FastC concerns:

| Benchmark | What it measures |
|-----------|-----------------|
| **array-sum** | Loop + accumulator with and without bounds checks |
| **struct-access** | Field access patterns, struct copy vs. reference |
| **bounds-check-overhead** | Same algorithm with `unsafe` (no checks) vs. safe code |
| **ffi-call** | Overhead of calling a C function from FastC and vice versa |

The **bounds-check-overhead** benchmark is unique to FastC — it directly quantifies the cost of safety, which is a key question for potential adopters.

### Agent Usability Benchmarks

These measure how well FastC serves AI coding agents compared to other languages. See [docs/agent-features.md](agent-features.md) for the full agent-first specification.

| Metric | Protocol |
|--------|----------|
| **Error recovery rate** | Give an agent broken code + compiler output. Measure how often it produces a fix in one round-trip. |
| **Code gen accuracy** | Give an agent a spec. Measure how often generated code compiles and passes tests on first try. |
| **Diagnostic parsability** | Feed compiler errors to an LLM. Measure whether it correctly identifies the file, line, and fix. |
| **Round-trip consistency** | `format → parse → format` produces identical output (already guaranteed by deterministic output). |

## Directory Structure

```
bench/
├── run_all.sh              # Orchestrator script
├── compare.py              # Results aggregation and table generation
├── README.md               # How to run benchmarks locally
├── results/                # Historical results (git-tracked)
│   └── YYYY-MM-DD-HH-MM/  # Timestamped result directories
│       ├── results.json
│       └── results.md
├── programs/               # CLBG programs
│   ├── nbody/
│   │   ├── nbody.fc
│   │   ├── nbody.c
│   │   ├── nbody.zig
│   │   ├── nbody.rs
│   │   └── Makefile
│   ├── binary-trees/
│   ├── spectral-norm/
│   ├── mandelbrot/
│   ├── fannkuch-redux/
│   └── fasta/
├── micro/                  # Micro-benchmarks
│   ├── array-sum/
│   ├── struct-access/
│   ├── bounds-check/
│   └── ffi-call/
└── agent/                  # Agent usability benchmarks
    ├── error-recovery/
    ├── codegen-accuracy/
    └── diagnostic-parse/
```

Each program directory contains idiomatic implementations in all compared languages plus a `Makefile` that builds and validates correctness (output must match a reference).

## Measurement Methodology

### Runtime Performance

- **Tool**: [hyperfine](https://github.com/sharkdp/hyperfine) with `--warmup 3 --min-runs 10`
- **Metrics**: wall-clock time (median, mean, stddev, min, max)
- **Environment**: isolated machine, CPU frequency pinned, no background load
- **Compilation**: each language compiled with its standard release/optimization flags:
  - FastC: `fastc build` → `cc -O2 -std=c11`
  - C: `gcc -O2 -std=c11` and `clang -O2 -std=c11`
  - Zig: `zig build -Doptimize=ReleaseFast`
  - Rust: `cargo build --release`

### Hardware Counters

- **Tool**: `perf stat` (Linux) or Instruments (macOS)
- **Metrics**: instructions retired, cache misses (L1d, LLC), branch mispredictions, cycles
- **Purpose**: explain *why* performance differs, not just *that* it differs

### Memory Usage

- **Tool**: `/usr/bin/time -v` (Linux) or `/usr/bin/time -l` (macOS)
- **Metrics**: peak RSS, page faults
- **Purpose**: measure allocation overhead from safety wrappers

### Binary Size

- **Tool**: `size` command on stripped binaries
- **Metrics**: text, data, bss, total
- **Purpose**: compare code bloat from monomorphization and safety checks

### Lines of Code

- **Tool**: [tokei](https://github.com/XAMPPRocky/tokei)
- **Metrics**: code lines (excluding comments and blanks)
- **Purpose**: measure expressiveness / verbosity trade-offs

### Compile Time

- **Tool**: `hyperfine` on full clean builds
- **Metrics**: wall-clock time for full pipeline
- **Comparison**: `fastc + cc` vs `gcc` vs `clang` vs `zig build` vs `cargo build --release`
- **Note**: FastC has an inherent two-stage cost (transpile + C compile). This is measured honestly as a combined time.

## Reporting

### Output Format

`compare.py` generates a markdown table for each benchmark category:

```
## n-body (N=50000000)

| Language | Time (ms) | σ | Instructions | Peak RSS (KB) | Binary (KB) | LOC |
|----------|-----------|---|-------------|----------------|-------------|-----|
| C (gcc)  |     1,234 | 5 | 12,345,678K |          1,200 |          12 |  85 |
| C (clang)|     1,198 | 4 | 12,100,000K |          1,200 |          14 |  85 |
| FastC    |     1,256 | 6 | 12,500,000K |          1,210 |          16 |  72 |
| Zig      |     1,201 | 5 | 12,200,000K |          1,205 |          10 |  90 |
| Rust     |     1,310 | 8 | 13,000,000K |          1,400 |          48 |  95 |

Hardware: AMD Ryzen 9 7950X, 64GB DDR5, Ubuntu 24.04, kernel 6.8
Compilers: gcc 14.1, clang 18.1, zig 0.13, rustc 1.79, fastc 1.1
```

### CI Integration

- Benchmarks run on tagged releases and weekly on `main`.
- Results are committed to `bench/results/` for historical tracking.
- Regressions > 5% trigger a CI warning (not failure — benchmarks are noisy).

## Anti-Patterns

These are things we explicitly avoid:

1. **Cherry-picking results.** All benchmark results are published, including ones where FastC is slower. If FastC loses on a benchmark, we explain why and track improvement.

2. **Unfair compilation flags.** All languages use their standard release optimization flags. No `-march=native` unless applied uniformly. No LTO unless applied uniformly.

3. **Unstable baselines.** Every result directory records exact compiler versions, kernel version, CPU model, and governor settings. Results without hardware specs are not published.

4. **Benchmarking toy programs.** CLBG programs are non-trivial and well-studied. We do not benchmark `hello_world` compilation time.

5. **Ignoring two-stage cost.** FastC transpiles to C, then compiles C. The total time is what matters to users, and that is what we report.

6. **Irreproducible results.** Anyone with the specified hardware and compiler versions should be able to reproduce results within statistical noise. The `run_all.sh` script handles all setup.
