# FastC

[![Build Status](https://github.com/fastc-lang/fastc/workflows/CI/badge.svg)](https://github.com/fastc-lang/fastc/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.85%2B-orange.svg)](https://www.rust-lang.org/)
[![Documentation](https://img.shields.io/badge/docs-online-green.svg)](https://docs.fastc-lang.com)

**A small systems language with capability-typed I/O, mandatory contracts, and zero executable build scripts — built for the age of agent-generated code.**

fastC is a modern C-like language for a world where most code is written by an AI agent and reviewed by a human. It compiles to readable C11. It refuses to run anything at build time. Capabilities (`fs.read`, `net.connect`, …) are typed function arguments — a function with no capability arguments structurally cannot do I/O. Pre- and postconditions on public APIs are compile-time obligations.

![demo](assets/demo.gif)

## Why fastC

| | C | Rust | Zig | Go | **fastC** |
|---|---|---|---|---|---|
| Memory safety without GC | ✗ | ✓ | partial | ✗ (GC) | **✓** |
| No executable build scripts | ✓ | ✗ `build.rs` | ✗ `build.zig` | ✗ `cgo` | **✓** |
| Capability-typed I/O | ✗ | ✗ | ✗ | ✗ | **✓** |
| Compile-time contracts (`@requires` / `@ensures`, SMT-discharged) | ✗ | ✗ | partial | ✗ | **✓ (three-tier: syntactic → Z3 → runtime)** |
| Outputs portable C11 | is C | ✗ | ✗ | ✗ | **✓** |
| Vendor-first deps (no central registry) | N/A | crates.io | Zon | modules | **✓** |
| Sigstore / SLSA provenance | ✗ | ✗ | ✗ | ✗ | **✓ (enforced sha256 + cosign keyless + SLSA L3)** |
| Stripped binary (hello) | 33 KB | 341 KB | 50 KB | 2.4 MB | **53 KB** |
| Cross-compile, no sysroot setup | depends on toolchain | rustup per-target dance | ✓ (`zig cc`, 50+) | ✓ (`GOOS`/`GOARCH`) | **✓ (8 presets via `zig cc`, any C cross-toolchain via `--cc-override`)** |

Each row has a paragraph of context at [docs.fastc-lang.com/why/rubric](https://docs.fastc-lang.com/why/rubric/). The honest framing of which trade-offs fastC actually wins on (and which it loses) is in [docs/MANIFESTO.md](docs/MANIFESTO.md).

## Measured numbers

[Benchmarks](https://docs.fastc-lang.com/why/benchmarks/) on M3:

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

For the inner edit/build/test loop, `fastc build --dev` swaps `cc -O2` for the fastest available C compiler at no-opt (`tcc` if installed, else `cc -O0`). Measured 252 ms → 160 ms on a hello project (36% faster) without tcc; on Linux/Intel-Mac with tcc available the C step drops to under 10 ms.

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

**Cross-compile, one flag.** `fastc build --target=<triple>` ships eight pre-wired presets via `zig cc` (aarch64/x86_64 × linux-musl/linux-gnu, aarch64/x86_64-macos, wasm32-wasi, riscv64-linux-musl). One `brew install zig` and `fastc build --target=aarch64-linux-musl` produces a statically-linked ARM Linux binary, or `--target=wasm32-wasi` a `.wasm` for sandboxed runtimes. `--cc-override=<path>` swaps in a proprietary toolchain when needed. fastC emits portable C11, so any C cross-compiler in the world targets fastC binaries — we just default to the best one. See [docs/cross-compile.md](docs/cross-compile.md) and `fastc target list` for the live matrix.

**fastc-core launch set (stage 1.8 preview).** Five batteries-included modules ship in the prelude: `mod cli` (argv + flag parsing), `mod log` (debug/info/warn/error + structured kv pairs), `mod json` (encoder + a `find_int` decoder slice for top-level fields), `mod toml` (flat-table `find_int` / `find_bool` for config files), and `mod http` (`get_status` over a real TCP socket, gated on `CapNetConnect`). The integrated demo at [examples/launch_set_demo.fc](examples/launch_set_demo.fc) uses all five from a single program — CLI flags → TOML default → log → cap-typed HTTP call → JSON field extract. The eventual split to standalone vendor packages (`fastc-core/cli`, `fastc-core/http`, …) is a 1.7 packaging change, not a code change; the surface is final.

**SMT contract discharge (stage 2.1 preview).** `fastc compile --prove` runs every `@requires` / `@ensures` clause through a three-tier pipeline: a syntactic discharger (constant-fold + tautological-comparison detection, always on, no Z3 dependency) catches the free wins; an opt-in Z3 tier handles linear-integer tautologies with a 500 ms per-obligation budget; anything tier-1 and tier-2 can't prove falls back to the existing runtime `fc_trap`. **Proven obligations cost zero at runtime.** The build emits a `discharge.json` per-obligation report listing proven/runtime/unknown counts plus tier. Z3-not-on-PATH degrades to runtime tier with a structured reason (no build break). See `examples/cli_demo.fc` for a working program with mixed proven and runtime clauses.

## Quick Start

```bash
# Install
git clone https://github.com/fastc-lang/fastc.git
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

# Add a vendored dependency — shows capabilities + content hash before writing
fastc add https://github.com/fastc-lang/fastc-core-http
# Records sha256 in fastc.toml + fastc.lock; subsequent builds verify the
# fetched cache against that hash and fail loudly on any drift.
```

## Status

**fastC is v1.0 feature-complete.** Stages 0.1 through 2.1 are shipped: capability-typed I/O with fabrication check + `caps.json` artifact, vendor-first dependencies with enforced sha256 + cosign keyless + SLSA L3, the eleven-package `fastc-core` set, cross-compilation to eight targets via `zig cc`, three-tier contract discharge (syntactic + Z3 + runtime), `[workspace]` manifest with per-member incremental, closures with literal captures, compiler-binary reproducibility, full v1.3 annotation surface (`@mem` / `@panics` / `@purity` / `@complexity`), module-level mandatory headers, `fastc fix` / `context` / `diff` / inline `test { }` blocks / unified JSON diagnostic envelope. **337+ tests pass** across the workspace.

The v1.0 stability commitment is documented in [docs/v1.0.md](docs/v1.0.md); the slice-by-slice history is in [docs/roadmap.md](docs/roadmap.md).

## fastc-core packages

Eleven public preview packages under [fastc-lang](https://github.com/fastc-lang):

| Package | Repo | Purpose |
|---|---|---|
| `cli` | [fastc-core-cli](https://github.com/fastc-lang/fastc-core-cli) | Argv access + flag parsing |
| `log` | [fastc-core-log](https://github.com/fastc-lang/fastc-core-log) | Structured leveled logging |
| `json` | [fastc-core-json](https://github.com/fastc-lang/fastc-core-json) | JSON encode + integer-field decode |
| `toml` | [fastc-core-toml](https://github.com/fastc-lang/fastc-core-toml) | Read-only flat-table TOML parser |
| `http` | [fastc-core-http](https://github.com/fastc-lang/fastc-core-http) | HTTP/1.1 client (CapNetConnect-gated) |
| `time` | [fastc-core-time](https://github.com/fastc-lang/fastc-core-time) | Wall-clock + ISO 8601 |
| `base64` | [fastc-core-base64](https://github.com/fastc-lang/fastc-core-base64) | RFC 4648 encode/decode |
| `uuid` | [fastc-core-uuid](https://github.com/fastc-lang/fastc-core-uuid) | RFC 4122 v4 + parse/format |
| `crypto-primitives` | [fastc-core-crypto-primitives](https://github.com/fastc-lang/fastc-core-crypto-primitives) | SHA-256, HMAC, constant-time compare |
| `regex` | [fastc-core-regex](https://github.com/fastc-lang/fastc-core-regex) | Thompson NFA, no backreferences |
| `sqlite` | [fastc-core-sqlite](https://github.com/fastc-lang/fastc-core-sqlite) | FFI bindings to libsqlite3 |

Each ships a `v0.1.0` preview release alongside fastC v1.0. The implementations currently live inside the compiler's built-in prelude; the public repos become installable via `fastc add` when the v1.1 vendor-consumption flow lands.

## Testing

The harness scripts in `scripts/` wrap the underlying `cargo test` so one command runs the whole suite with colorized output:

```bash
bash scripts/test.sh           # full: build + tests + format + examples smoke
bash scripts/test.sh quick     # ~20s — unit + integration only
bash scripts/test.sh ci        # mirror what CI runs (fmt + clippy + tests)
bash scripts/check.sh          # alias for `test.sh ci` — pre-PR check
bash scripts/bench.sh          # cross-language benchmark suite
```

See [`scripts/README.md`](scripts/README.md) for what each mode covers.

## Documentation

- [**Why fastC**](https://docs.fastc-lang.com/why/) — rubric, benchmarks, FAQs.
- [**Getting Started**](https://docs.fastc-lang.com/getting-started/) — installation, first project.
- [**Language Guide**](https://docs.fastc-lang.com/language/) — type system, syntax, idioms.
- [**C Interop**](https://docs.fastc-lang.com/c-interop/) — calling C, exposing fastC APIs to C.
- [**CLI Reference**](https://docs.fastc-lang.com/cli/) — every command and flag.

## Editor support

Install the language server: `cargo install --path crates/fastc-lsp`. Wire it up to your editor; see [Editor Setup](https://docs.fastc-lang.com/getting-started/editor-setup/).

## FAQ

### What is fastC?

fastC is a small systems programming language with capability-typed I/O, mandatory contracts, and zero executable build scripts. It compiles to portable, readable C11, ships 53 KB stripped binaries, and is built for a world where most code is written by an AI agent and reviewed by a human. It is MIT-licensed and feature-complete at v1.0.

### What does "agent-first" (or "agent-friendly") mean?

An agent-first language is designed around the fact that the modal author of code is now a stochastic model, not a careful human. fastC is agent-first in four concrete ways: a function's signature declares its capabilities, contracts, purity, and complexity, so an agent can read what a function may do without reading its body; diagnostics are one structured JSON envelope instead of prose to parse; the toolchain speaks Model Context Protocol (MCP) natively via `fastc mcp`; and the compiler refuses whole classes of mistake rather than miscompiling them. The compiler is the only deterministic step between a stochastic author and a stochastic reviewer.

### What are capabilities in fastC?

The authority to do I/O is a typed value passed as a function argument, not ambient. Capabilities like `CapFsRead`, `CapNetConnect`, and `CapProcSpawn` are a finite, named set, minted only in `main` via `caps::init()` and flowed downward through call arguments. A function whose signature carries no capability structurally cannot read files, open sockets, or spawn processes — the compiler refuses the call. This turns "can this code exfiltrate data?" into a signature-reading exercise. See [docs.fastc-lang.com/language/capabilities/](https://docs.fastc-lang.com/language/capabilities/).

### What are contracts in fastC?

Contracts are pre- and postconditions — `@requires` and `@ensures` — that fastC treats as compile-time obligations on public APIs, discharged through three tiers: a syntactic pass proves the free cases, Z3 proves linear-integer tautologies under a 500 ms per-obligation budget, and anything unproven falls back to a runtime `fc_trap`. Proven obligations cost zero at runtime. See [docs.fastc-lang.com/language/contracts/](https://docs.fastc-lang.com/language/contracts/).

### Does fastC work with Claude, Cursor, Copilot, and other AI coding tools?

Yes. fastC ships `fastc mcp`, a Model Context Protocol server that exposes the compiler's structured artifacts — capabilities, contracts, diagnostics, and the module graph — to Claude Code, Cursor, Codex, and any MCP-capable agent. Agents read `fastc explain --json` and `fastc context` instead of text-parsing compiler output, and `fastc fix` applies the compiler's own fix-it hints. See [docs.fastc-lang.com/cli/](https://docs.fastc-lang.com/cli/).

### How does fastC compare to Rust, Zig, C, C++, Go, Python, and TypeScript?

fastC is the only language that combines capability-typed I/O, mandatory contracts, zero executable build scripts, vendored dependencies with enforced provenance (sha256 + cosign + SLSA L3), and a CI-enforced compile-time budget. Rust wins on borrow-checked safety and ecosystem; Zig wins on cross-compilation and C interop; C and C++ win on ubiquity and maturity but are unsafe by default; Go wins on goroutines but ships 2.4 MB binaries with a GC; Python and TypeScript win on ecosystem and agent familiarity but are interpreted/managed and carry the PyPI/npm install-script threat model. fastC's niche is small, safe, native binaries an agent writes and a human audits. Side-by-side tables are at [www.fastc-lang.com/compare/](https://www.fastc-lang.com/compare/).

### Why not just use opinionated Rust (cargo-vet, no proc macros, no `build.rs`, no async)?

Opinionated Rust is a moving target negotiated with a 150K-crate ecosystem that already chose differently. Even with strict project policy, you still inherit Rust's compile times, monomorphization fan-out, lifetime-annotation tax, and a stdlib that grew up around `Box<dyn>` and async. fastC is not "Rust minus features" — it is a smaller language designed *from the start* around four properties Rust cannot retrofit without breaking its ecosystem: (1) capabilities as typed function arguments, not ambient authority; (2) mandatory contracts on public APIs; (3) a package system with no executable build steps and no central registry, only content-hashed vendored deps; and (4) a compile-time budget enforced in CI. If your team can credibly enforce all of that on top of Rust forever, you should — Rust is a fine language. fastC is for the case where you cannot, or where you want the enforcement to come from the language rather than the policy.

### Is fastC production ready?

fastC is v1.0 feature-complete with 337+ tests, an MIT license, and Sigstore + SLSA L3 provenance on releases. The language surface and the 11-package `fastc-core` standard library are final for the v1.x line. Async/await (stage 2.3) and SMT-first contract discharge (stage 2.1) are deferred to later milestones and documented as such.

More on the design rationale — C interop and safety defaults — lives at [docs.fastc-lang.com/why/](https://docs.fastc-lang.com/why/).

## License

MIT. See [LICENSE](LICENSE).

---

<p align="center">
  <a href="https://github.com/fastc-lang/fastc">GitHub</a> ·
  <a href="https://docs.fastc-lang.com">Documentation</a> ·
  <a href="https://github.com/fastc-lang/fastc/issues">Issues</a>
</p>
