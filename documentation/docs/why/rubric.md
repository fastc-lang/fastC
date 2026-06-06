# Rubric — fastC vs C, Rust, Zig, Go

| Dimension | C | Rust | Zig | Go | **fastC** |
|---|---|---|---|---|---|
| Memory safety (without GC) | ✗ | ✓ | partial | ✗ (GC) | **✓** |
| No executable build scripts | ✓ | ✗ `build.rs` | ✗ `build.zig` | ✗ `cgo` | **✓ declarative only** |
| Capability-typed I/O | ✗ | ✗ | ✗ | ✗ | **✓ in type system** |
| Compile-time contracts | ✗ | ✗ | partial (`comptime`) | ✗ | **✓ `@requires` / `@ensures` (SMT-discharged via Z3 + three-tier pipeline)** |
| Outputs portable C11 | (is C) | ✗ | ✗ | ✗ | **✓** |
| Central package registry | N/A | crates.io | Zon | Go modules | **none (vendor-first)** |
| Sigstore / SLSA provenance | ✗ | ✗ | ✗ | ✗ | **✓ (enforced sha256 + cosign keyless + SLSA L3)** |
| Native MCP server | ✗ | ✗ | ✗ | ✗ | **✓ `fastc-mcp`** |
| Mandatory module annotations | ✗ | ✗ | ✗ | ✗ | **✓ (`@owns` / `@arch` / …)** |
| Agent tooling surface | ✗ | ✗ | ✗ | ✗ | **✓ (`fastc fix` / `context` / `diff` / `mcp` / `explain`)** |
| Function-level annotation surface | ✗ | partial (attributes, no enforcement) | partial | partial | **✓ (`@purity` / `@panics` / `@complexity` + module headers, two enforced)** |
| Supply chain provenance | ✗ | partial (crates.io account model) | partial | ✓ (`go.sum`) | **✓ (sha256 + cosign keyless + SLSA L3 + `dep_content_hash` in cache key)** |
| Curated stdlib extensions | N/A | ✗ (150K-crate ecosystem) | ✓ (stdlib only) | ✓ (broad stdlib) | **✓ (11-package `fastc-core` ecosystem)** |
| Binary size (stripped, hello) | 33 KB | 341 KB | 50 KB | 2.4 MB | **53 KB** |
| Cross-compile targets, no setup | depends on toolchain | ~200 (rustup, per-target sysroot dance) | 50+ (`zig cc`, bundled libcs) | 50+ (`GOOS`/`GOARCH`) | **8 shipped presets via `zig cc`, plus any C cross-toolchain via `--cc-override`** |

## How to read this table

Each row is a dimension where fastC made a deliberate choice. The columns show what each comparable language does today. Rows where fastC stands alone with a single ✓ vs four ✗s are where the strategic wedge lives — that's the work the other four languages haven't done.

### Memory safety without GC

Rust and fastC are the only mainstream systems languages that prevent use-after-free and buffer overruns at compile time without paying for a garbage collector. Zig has partial safety in Debug mode (the GeneralPurposeAllocator catches leaks and double-frees) but not in ReleaseFast. C and Go either pay the runtime cost (Go's GC) or accept undefined behavior on misuse (C).

### No executable build scripts

This is the single biggest supply-chain attack vector in the Rust / npm / Zig ecosystems. A malicious `build.rs` runs with the privileges of whoever invoked `cargo build`. Concrete 2025/2026 incidents (`faster_log`, `async_println`, CVE-2026-28353) all leveraged this. fastC's manifest is TOML only — there is no place to put code.

### Capability-typed I/O

A `fn` in C, Rust, Zig, or Go that wants to read a file just calls `open()`. The type system has no idea this happened. In fastC, the type system *does* know — `fs::exists(c: ref(CapFsRead), path)` is the only way in, and the cap is minted in `main` and passed downward explicitly. A function with no cap arguments cannot touch the filesystem. This is the wedge.

### Compile-time contracts

`@requires(x > 0)` is parsed and lowered to `if (!cond) fc_trap();` at function entry. `@ensures(result >= 0)` is checked at every return site. **Stage 2.1 shipped a three-tier discharge pipeline**: tier-1 syntactic (constant-fold + tautology detection, always on) catches `@requires(true)` and similar trivia for free; tier-2 SMT (shells out to `z3`, opt-in via `--prove`) handles linear-integer tautologies like `(a > 0) || (a == 0) || (a < 0)`; tier-3 runtime is the safe fallback. **Proven obligations elide their `fc_trap` guard — zero runtime cost.** No other production language ships this in 2026 — SPARK Ada and F* have it but neither has the syntax fastC has, and neither compiles to C. The per-build `discharge.json` report makes the proven/runtime/unknown split auditable.

### Outputs portable C11

fastC emits readable, deterministic C11 that compiles with any C compiler. Your existing toolchain — gdb, perf, valgrind, ASan, your CI's gcc — all still work. Rust, Zig, Go either own the whole toolchain or partially override it.

### Vendor-first, no central registry

Crates.io, npm, and PyPI are the typosquat and phishing targets that have driven the 2024–2026 supply-chain wave. fastC dependencies are git URL + commit hash + content sha256 + Sigstore bundle. There's nothing to compromise on a registry server.

### Native MCP server

`fastc-mcp` exposes the compiler's AST, capability graph, and contract discharge over Model Context Protocol. Claude Code, Cursor, Codex talk to it natively — no `cargo check` text parsing, no fragile JSON wrappers around toolchain output. The compiler is the agent's interface.

### Mandatory module annotations

Every fastC module declares `@owns`, `@arch`, `@depends`, `@threading`, `@invariants` in a header. The compiler checks these are present and consistent. An agent (or human) reading a fastC module gets the architectural context for free — no archaeology through git history required.

### Agent tooling surface

`fastc fix` applies the compiler's fix-it spans mechanically (`wrap in unsafe`, `add addr(`, `import X`); `fastc context` dumps the project's pub type surface as markdown or JSON for AI context windows; `fastc diff` produces an AST-level semantic diff between two snapshots (added / removed / signature-changed pub items); `fastc explain` emits per-function JSON (signature + caps + contracts + v1.3 annotations + module headers); `fastc mcp` is a stdio JSON-RPC server exposing five tools (`explain`, `check`, `compile`, `caps_summary`, `context`, `diff`) over Model Context Protocol. The other four languages ship none of this — agent tools text-scrape `cargo check` / `zig build` / `go vet` output instead.

### Function-level annotation surface

fastC v1.3 ships `@purity(pure | effect | io)`, `@panics(never | always | on=expr)`, `@complexity(O(...))`, and `@mem(arena=ident)` as first-class function annotations. **`@panics(never)` and `@purity(pure)` are enforced** against the transitive call graph (no path to `fc_trap` / `panic` / `abort` / `exit`; no alloc / I/O / logging). `@complexity` and `@mem` parse and round-trip through `fastc explain` for now. Plus module-level `//! @module / @owns / @arch / @depends / @threading / @invariants` headers with cross-module checks (uniqueness, dependency exhaustiveness, arch DAG layering). Rust has attributes (`#[must_use]`, `#[inline]`) but no purity / panic / complexity surface; Zig and Go have a partial story via `comptime` / linter rules; C has nothing.

### Supply chain provenance

The fastC release pipeline (`.github/workflows/release.yml`) ships every compiler binary with a cosign keyless signature and SLSA L3 provenance. Every dep in a fastC project records a `sha256` content hash in `fastc.lock`, verified on every `fastc fetch` — a tampered cache tree fails the build before any source compiles. **The `dep_content_hash` is part of the build cache key**, so dep churn invalidates the cache by construction (no silent stale-cache builds). Go has `go.sum` (closest equivalent); Rust has crates.io's account model; Zig has Zon hashes but the SLSA / cosign story is still building out; C has nothing structural.

### Curated stdlib extensions

fastC ships an **11-package `fastc-core` ecosystem** under [Skelf-Research](https://github.com/Skelf-Research) — `cli`, `log`, `json`, `toml`, `http`, `time`, `base64`, `uuid`, `crypto-primitives`, `regex`, `sqlite`. Each is a separate public repo with its own `fastc.toml` / `README.md` / `AGENTS.md` / `LICENSE`. The implementations ship inside the v1.0 prelude (no per-user vendor cutover yet); the v1.1 packaging slice moves them onto the `fastc add` flow. Rust's 150K-crate registry is a different problem (typosquats, ungated tier-3 transitive deps); fastC's curated 11-package set is the deliberate small-and-audited counterposition. Zig and Go both rely on stdlib breadth.

### Binary size

A stripped `hello` binary measured on M3 / macOS 25.4:

- C: 33 KB (gcc -O2)
- Zig: 50 KB (zig build-exe -O ReleaseFast -lc)
- **fastC: 53 KB** (fastc compile + cc -O2)
- Rust: 341 KB (rustc -O)
- Go: 2.4 MB (go build)

fastC is in the C / Zig binary-size class. Rust is **6.4× larger**; Go is **45× larger**. The fastC vs C delta (~20 KB) is the runtime header that delivers fastC's safety guarantees in compiled output. The fastC vs Zig delta is single-digit kilobytes — essentially the same class.

Why this column matters: container cold-start, embedded ceilings, distribution / audit costs all scale with binary size. fastC's structural choice to ship a tiny static-inline runtime in a single header (rather than a large standard library) is what makes 53 KB binaries achievable at all. See [benchmarks](benchmarks.md#binary-size-stripped--fastc-is-in-the-c--zig-class-not-the-rust--go-class) for the full per-program table and the ratio analysis.

### Cross-compile targets, no setup

fastC ships eight pre-wired target presets in v1.9 — aarch64/x86_64 × linux-musl/linux-gnu, aarch64/x86_64-macos, wasm32-wasi, and riscv64-linux-musl — covering cloud / Apple Silicon / sandboxed WASM / RISC-V. They all go through `zig cc`, so a single `brew install zig` is the only setup. Run `fastc target list` for the live matrix; run `fastc build --target=<triple>` to produce a binary; run `fastc target check <triple>` to verify the backend without compiling.

The strategic claim is structural, not numerical: **fastC emits portable C11, which means every C cross-compiler in the world is a fastC cross-compiler.** We default to zig because it's the best one and ships with bundled libcs, but `--cc-override=<path>` plugs in any other toolchain (proprietary embedded compilers, distro gcc-cross, custom musl-cross). fastC inherits its cross-compile breadth from the underlying C toolchain — we don't compete with Zig on cross-compilation; we wrap it. See [`fastc target`](../cli/target.md) for the v1.9 target matrix and the `--cc-override` escape hatch.

## fastC vs Zig specifically

Zig is fastC's closest competitor by the empirical numbers — same binary-size class (~50 KB), same refusal to ship silently-wrong code (Zig's strict signed-division rule caught T5's overflow exactly as fastC's strict syntax caught it). The two languages will get compared side-by-side. The honest read on where each wins:

### Where fastC wins over Zig

1. **Capability-typed I/O.** A Zig function `pub fn process() void` can call `std.fs.cwd().openFile(...)`, `std.process.Child.spawn(...)`, or open a network socket, and its signature reveals nothing. fastC's `fn process(c: ref(CapFsRead)) -> i32` structurally cannot reach the filesystem without the cap declared at the type level. Zig has no equivalent and no plans to add one. This is the property the MANIFESTO leads with.

2. **No executable build scripts.** Zig has `build.zig` — arbitrary code that runs at `zig build` time. The 2025–2026 supply-chain incident class (faster_log, async_println, evm-units, CVE-2026-28353) applies to Zig package authors who ship malicious `build.zig` files. fastC's `fastc.toml` is closed-schema TOML enforced by `#[serde(deny_unknown_fields)]` — there is no syntactic place to put executable code. See `examples/supply_chain_demo/` for the side-by-side demo.

3. **Mandatory contracts on public APIs.** Zig has nothing comparable to `@requires` / `@ensures`. Assertions in Zig are runtime-only via `std.debug.assert`. fastC's contracts are compile-time obligations now and SMT-discharged in v2.1.

4. **Compiles to portable C11.** Zig emits its own object format; fastC outputs readable C11. That means gdb, perf, valgrind, ASan, every C compiler optimization, and every C audit tool work on fastC binaries directly. You can drop fastC into a C-only codebase without an FFI shim — the output IS C. Zig requires you to live inside the Zig toolchain.

5. **Mandatory module-header annotations.** `@owns` / `@arch` / `@depends` / `@threading` / `@invariants` are compiler-checked in fastC. Every module declares its architectural contract at the top of the file. Zig has no such convention; reviewers grep for context.

6. **Vendor-first dependencies with enforced sha256 + Sigstore (shipped, stage 1.7).** Zig has the Zon registry. fastC has no central registry — every dep is git URL + commit + sha256 + optional Sigstore bundle. The `sha256` is *enforced* on every `fastc fetch`: a tampered cache tree fails the build with the expected/got diagnostic before any source compiles. `fastc lock` anchors the lockfile; `fastc add` shows the dep's capability surface before you accept it. Compiler binaries themselves ship with cosign keyless signatures and SLSA L3 provenance via `.github/workflows/release.yml`. There's nothing to compromise on a registry server because there is no registry server.

7. **Native MCP server.** Zig agent tools text-scrape `zig build` output. fastC ships `fastc-mcp` as a first-class agent interface — AST, types, caps, contracts served over Model Context Protocol without text parsing.

8. **NASA/JPL Power-of-10 enforcement.** `fastc check --safety-level=critical` enforces no-recursion, no-allocation, bounded-loops, function-size limits. Zig has no equivalent CLI gate.

### Where Zig wins over fastC

1. **Compile time.** Zig 149 ms vs fastC 215 ms on the four-program benchmark — Zig is ~30% faster end-to-end. fastC's planned tcc dev-mode backend (stage 0.8) targets sub-200 ms but hasn't shipped.

2. **Maturity.** Zig 0.16 is shipping with a real library ecosystem. fastC is pre-1.0 with a sub-thousand-package world.

3. **C ingestion.** Zig's `@cImport` consumes any C header automatically. fastC requires hand-written `extern "C"` blocks. The deliberate trade is auditability (every external C symbol fastC touches is enumerated and reviewable), but it's a real ergonomic loss.

4. **`comptime`.** Zig's compile-time metaprogramming is genuinely powerful — generic containers without monomorphization explosions, embedded DSLs, build-time codegen. fastC has no equivalent; we trade expressive power for predictable codegen and a smaller language surface.

5. **Cross-compilation breadth.** Zig ships 50+ targets in one binary. fastC ships eight pre-wired targets and routes them through `zig cc` (so the underlying capability is the same), with `--cc-override` for proprietary toolchains. Zig still wins on out-of-the-box breadth — fastC's set is curated to where it plausibly competes (cloud / Apple Silicon / WASI / RISC-V). See [`fastc target`](../cli/target.md) for the v1.9 matrix.

### One-sentence positioning

Zig and fastC made opposite choices about what to push to the language level. Zig prioritized performance and metaprogramming (`comptime`, cross-compilation, C ingestion). fastC prioritized safety and auditability (capability-typed I/O, declarative manifests, mandatory contracts, MCP-native diagnostics). On the dimensions both share — binary size, no-GC native code, refusing silent UB — they're tied. The wedge is everywhere else.
