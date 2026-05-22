# FastC

[![Build Status](https://github.com/Skelf-Research/fastc/workflows/CI/badge.svg)](https://github.com/Skelf-Research/fastc/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.85%2B-orange.svg)](https://www.rust-lang.org/)
[![Documentation](https://img.shields.io/badge/docs-online-green.svg)](https://docs.skelfresearch.com/fastc)

**A small systems language with capability-typed I/O, mandatory contracts, and zero executable build scripts — built for the age of agent-generated code.**

fastC is a modern C-like language for a world where most code is written by an AI agent and reviewed by a human. It compiles to readable C11. It refuses to run anything at build time. Capabilities (`fs.read`, `net.connect`, …) are typed function arguments — a function with no capability arguments structurally cannot do I/O. Pre- and postconditions on public APIs are compile-time obligations.

![demo](assets/demo.gif)

## Why fastC

| | C | Rust | Zig | Go | **fastC** |
|---|---|---|---|---|---|
| Memory safety without GC | ✗ | ✓ | partial | ✗ (GC) | **✓** |
| No executable build scripts | ✓ | ✗ `build.rs` | ✗ `build.zig` | ✗ `cgo` | **✓** |
| Capability-typed I/O | ✗ | ✗ | ✗ | ✗ | **✓** |
| Compile-time contracts (`@requires` / `@ensures`) | ✗ | ✗ | partial | ✗ | **✓** |
| Outputs portable C11 | is C | ✗ | ✗ | ✗ | **✓** |
| Vendor-first deps (no central registry) | N/A | crates.io | Zon | modules | **✓** |
| Sigstore / SLSA provenance | ✗ | ✗ | ✗ | ✗ | **scheduled** |
| Stripped binary (hello) | 33 KB | 341 KB | 50 KB | 2.4 MB | **53 KB** |

Each row has a paragraph of context at [docs.skelfresearch.com/fastc/why/rubric](https://docs.skelfresearch.com/fastc/why/rubric). The honest framing of which trade-offs fastC actually wins on (and which it loses) is in [docs/MANIFESTO.md](docs/MANIFESTO.md).

## Measured numbers

[Benchmarks](https://docs.skelfresearch.com/fastc/why/benchmarks) on M3:

**Binary size, stripped** — the headline. fastC is in the C / Zig class, not Rust / Go:

| Lang | hello | sum | fib40 | mandelbrot | vs fastC |
|---|---|---|---|---|---|
| C | 33 KB | 17 KB | 17 KB | 33 KB | 0.3–0.6× |
| Zig | 50 KB | 50 KB | 50 KB | 50 KB | 0.95× |
| **fastC** | **53 KB** | **53 KB** | **53 KB** | **53 KB** | 1.0× |
| Rust | 342 KB | 341 KB | 341 KB | 342 KB | **6.4× larger** |
| Go | 2.4 MB | 2.1 MB | 2.1 MB | 2.1 MB | **40× larger** |

A fastC binary that does real work fits inside the cold-start budget of every container platform, every embedded device with ≥ 64 KB of flash, and every audit-by-disassembly workflow. Rust and Go binaries do not.

**Compile time + runtime** (snapshot 2026-05-22):

| Program | fastC compile | fastC runtime | C runtime (gcc -O2) |
|---|---|---|---|
| hello | 215ms | 1ms | 2ms |
| sum (1..1M loop) | 215ms | 3ms | 3ms |
| fib(40) | 217ms | 445ms | 354ms |
| mandelbrot 800×800 | 218ms | 63ms | 62ms |

fastC compile time is ~30–40% faster than Rust to a release binary. Runtime matches C on FP-heavy work; 26% slower on recursive integer (overflow-check cost).

**Agent first-compile success** on T1 sum_array, 4 Ollama Cloud open-weight LLMs × N=3 trials per cell:

| Lang | GLM | Kimi | DeepSeek | Qwen |
|---|---|---|---|---|
| C | 3/3 | 3/3 | 3/3 | 3/3 |
| Rust | 3/3 | 2/3 | 2/2 | 2/2 |
| Zig | 3/3 | 2/2 | 3/3 | 0/2 |
| Go | 3/3 | 1/1 | TBD | TBD |
| **fastC** | **3/3** | **3/3** | **3/3** | **3/3** |

fastC matches or beats every other language on T1 — **once the cheatsheet shipped with the prompt is faithful**. An earlier run against an inaccurate cheatsheet scored 0/9; rewriting the cheatsheet around a verified worked example and a "common mistakes" inverse guide flipped the result to 12/12.

**Safety wedge** on T5 large_sum (sum 1..100000 without overflow warning), GLM N=3:

| Lang | compiled | correct | silently-wrong |
|---|---|---|---|
| C | 3/3 | 3/3 | 0 |
| Rust | 3/3 | 1/3 | 2 (silent integer wrap) |
| Go | 3/3 | 0/3 | 3 (silent integer wrap) |
| Zig | 0/3 | 0/3 | 0 (refused: signed `/` needs `@divTrunc`) |
| fastC | 3/3 | 3/3 | 0 |

Go silently wrapped 3/3; Rust 2/3. fastC and Zig either refused to compile or computed correctly — neither shipped a silently-wrong binary.

The scripts and golden data are in [`benchmarks/cross-lang/`](benchmarks/cross-lang/). Re-running the perf suite takes 30 seconds; the first-compile-success harness against the four Ollama models with a single key is ~$2 and ~30 minutes. See [`benchmarks/cross-lang/first-compile/`](benchmarks/cross-lang/first-compile/).

The [supply-chain side-by-side demo](examples/supply_chain_demo/) shows `cargo build` executing a malicious `build.rs` vs `fastc.toml` rejecting the same shape at parse time.

## Quick Start

```bash
# Install
git clone https://github.com/Skelf-Research/fastc.git
cd fastc
cargo install --path crates/fastc

# Hello world
echo 'fn main() -> i32 { return 0; }' > hello.fc
fastc compile hello.fc -o hello.c
cc hello.c -o hello && ./hello && echo OK

# Or scaffold a project
fastc new my_project
cd my_project
fastc run
```

## Status

Stage 1.5 (contracts, runtime tier) is the latest checkpoint. 222 tests pass across the workspace. The read-side capability-typed I/O surfaces (`mod time` / `mod env` / `mod rand` / `mod fs`) are all live and exercised end-to-end. Closures-with-captures, SMT contract discharge, Sigstore fetch-side enforcement are the remaining multi-week items for v1.0.

See [`docs/roadmap.md`](docs/roadmap.md) for the slice-by-slice history.

## Documentation

- [**Why fastC**](https://docs.skelfresearch.com/fastc/why/) — rubric, benchmarks, FAQs.
- [**Getting Started**](https://docs.skelfresearch.com/fastc/getting-started/) — installation, first project.
- [**Language Guide**](https://docs.skelfresearch.com/fastc/language/) — type system, syntax, idioms.
- [**C Interop**](https://docs.skelfresearch.com/fastc/c-interop/) — calling C, exposing fastC APIs to C.
- [**CLI Reference**](https://docs.skelfresearch.com/fastc/cli/) — every command and flag.

## Editor support

Install the language server: `cargo install --path crates/fastc-lsp`. Wire it up to your editor; see [Editor Setup](https://docs.skelfresearch.com/fastc/getting-started/editor-setup/).

## FAQ — Why not opinionated Rust?

Why not Rust with cargo-vet, no proc macros, no `build.rs`, no async?

The honest answer is that opinionated Rust is a moving target negotiated with a 150K-crate ecosystem that already chose differently. Even with strict project policy, you still inherit Rust's compile times, monomorphization fan-out, lifetime-annotation tax, and a stdlib that grew up around `Box<dyn>` and async.

fastC is not "Rust minus features." It is a smaller language designed *from the start* around four properties Rust cannot retrofit without breaking its ecosystem:

1. Capabilities as typed function arguments, not ambient authority.
2. Mandatory contracts on public APIs, lowered to runtime asserts and (later) SMT-proven.
3. A package system with no executable build steps and no central registry, only content-hashed vendored deps.
4. A compile-time budget that is *enforced in CI*, not aspirational.

If your team can credibly enforce all of the above on top of Rust, you should — Rust is a fine language. fastC is for the case where you cannot.

Two more FAQs (C interop and safety defaults) live at [docs.skelfresearch.com/fastc/why/](https://docs.skelfresearch.com/fastc/why/).

## License

MIT. See [LICENSE](LICENSE).

---

<p align="center">
  <a href="https://github.com/Skelf-Research/fastc">GitHub</a> ·
  <a href="https://docs.skelfresearch.com/fastc">Documentation</a> ·
  <a href="https://github.com/Skelf-Research/fastc/issues">Issues</a>
</p>
