# Rubric — fastC vs C, Rust, Zig, Go

| Dimension | C | Rust | Zig | Go | **fastC** |
|---|---|---|---|---|---|
| Memory safety (without GC) | ✗ | ✓ | partial | ✗ (GC) | **✓** |
| No executable build scripts | ✓ | ✗ `build.rs` | ✗ `build.zig` | ✗ `cgo` | **✓ declarative only** |
| Capability-typed I/O | ✗ | ✗ | ✗ | ✗ | **✓ in type system** |
| Compile-time contracts | ✗ | ✗ | partial (`comptime`) | ✗ | **✓ `@requires` / `@ensures`** |
| Outputs portable C11 | (is C) | ✗ | ✗ | ✗ | **✓** |
| Central package registry | N/A | crates.io | Zon | Go modules | **none (vendor-first)** |
| Sigstore / SLSA provenance | ✗ | ✗ | ✗ | ✗ | **✓ (scheduled)** |
| Native MCP server | ✗ | ✗ | ✗ | ✗ | **✓ `fastc-mcp`** |
| Mandatory module annotations | ✗ | ✗ | ✗ | ✗ | **✓ (`@owns` / `@arch` / …)** |
| Binary size (stripped, hello) | 33 KB | 341 KB | 50 KB | 2.4 MB | **53 KB** |

## How to read this table

Each row is a dimension where fastC made a deliberate choice. The columns show what each comparable language does today. Rows where fastC stands alone with a single ✓ vs four ✗s are where the strategic wedge lives — that's the work the other four languages haven't done.

### Memory safety without GC

Rust and fastC are the only mainstream systems languages that prevent use-after-free and buffer overruns at compile time without paying for a garbage collector. Zig has partial safety in Debug mode (the GeneralPurposeAllocator catches leaks and double-frees) but not in ReleaseFast. C and Go either pay the runtime cost (Go's GC) or accept undefined behavior on misuse (C).

### No executable build scripts

This is the single biggest supply-chain attack vector in the Rust / npm / Zig ecosystems. A malicious `build.rs` runs with the privileges of whoever invoked `cargo build`. Concrete 2025/2026 incidents (`faster_log`, `async_println`, CVE-2026-28353) all leveraged this. fastC's manifest is TOML only — there is no place to put code.

### Capability-typed I/O

A `fn` in C, Rust, Zig, or Go that wants to read a file just calls `open()`. The type system has no idea this happened. In fastC, the type system *does* know — `fs::exists(c: ref(CapFsRead), path)` is the only way in, and the cap is minted in `main` and passed downward explicitly. A function with no cap arguments cannot touch the filesystem. This is the wedge.

### Compile-time contracts

`@requires(x > 0)` is parsed and lowered to `if (!cond) fc_trap();` at function entry. `@ensures(result >= 0)` is checked at every return site. In v2.1 these go to an SMT solver and what can be proven becomes free at runtime. No other production language ships this in 2026 — SPARK Ada and F* have it but neither has the syntax fastC has, and neither compiles to C.

### Outputs portable C11

fastC emits readable, deterministic C11 that compiles with any C compiler. Your existing toolchain — gdb, perf, valgrind, ASan, your CI's gcc — all still work. Rust, Zig, Go either own the whole toolchain or partially override it.

### Vendor-first, no central registry

Crates.io, npm, and PyPI are the typosquat and phishing targets that have driven the 2024–2026 supply-chain wave. fastC dependencies are git URL + commit hash + content sha256 + Sigstore bundle. There's nothing to compromise on a registry server.

### Native MCP server

`fastc-mcp` exposes the compiler's AST, capability graph, and contract discharge over Model Context Protocol. Claude Code, Cursor, Codex talk to it natively — no `cargo check` text parsing, no fragile JSON wrappers around toolchain output. The compiler is the agent's interface.

### Mandatory module annotations

Every fastC module declares `@owns`, `@arch`, `@depends`, `@threading`, `@invariants` in a header. The compiler checks these are present and consistent. An agent (or human) reading a fastC module gets the architectural context for free — no archaeology through git history required.

### Binary size

A stripped `hello` binary measured on M3 / macOS 25.4:

- C: 33 KB (gcc -O2)
- Zig: 50 KB (zig build-exe -O ReleaseFast -lc)
- **fastC: 53 KB** (fastc compile + cc -O2)
- Rust: 341 KB (rustc -O)
- Go: 2.4 MB (go build)

fastC is in the C / Zig binary-size class. Rust is **6.4× larger**; Go is **45× larger**. The fastC vs C delta (~20 KB) is the runtime header that delivers fastC's safety guarantees in compiled output. The fastC vs Zig delta is single-digit kilobytes — essentially the same class.

Why this column matters: container cold-start, embedded ceilings, distribution / audit costs all scale with binary size. fastC's structural choice to ship a tiny static-inline runtime in a single header (rather than a large standard library) is what makes 53 KB binaries achievable at all. See [benchmarks](benchmarks.md#binary-size-stripped--fastc-is-in-the-c--zig-class-not-the-rust--go-class) for the full per-program table and the ratio analysis.

## fastC vs Zig specifically

Zig is fastC's closest competitor by the empirical numbers — same binary-size class (~50 KB), same refusal to ship silently-wrong code (Zig's strict signed-division rule caught T5's overflow exactly as fastC's strict syntax caught it). The two languages will get compared side-by-side. The honest read on where each wins:

### Where fastC wins over Zig

1. **Capability-typed I/O.** A Zig function `pub fn process() void` can call `std.fs.cwd().openFile(...)`, `std.process.Child.spawn(...)`, or open a network socket, and its signature reveals nothing. fastC's `fn process(c: ref(CapFsRead)) -> i32` structurally cannot reach the filesystem without the cap declared at the type level. Zig has no equivalent and no plans to add one. This is the property the MANIFESTO leads with.

2. **No executable build scripts.** Zig has `build.zig` — arbitrary code that runs at `zig build` time. The 2025–2026 supply-chain incident class (faster_log, async_println, evm-units, CVE-2026-28353) applies to Zig package authors who ship malicious `build.zig` files. fastC's `fastc.toml` is closed-schema TOML enforced by `#[serde(deny_unknown_fields)]` — there is no syntactic place to put executable code. See `examples/supply_chain_demo/` for the side-by-side demo.

3. **Mandatory contracts on public APIs.** Zig has nothing comparable to `@requires` / `@ensures`. Assertions in Zig are runtime-only via `std.debug.assert`. fastC's contracts are compile-time obligations now and SMT-discharged in v2.1.

4. **Compiles to portable C11.** Zig emits its own object format; fastC outputs readable C11. That means gdb, perf, valgrind, ASan, every C compiler optimization, and every C audit tool work on fastC binaries directly. You can drop fastC into a C-only codebase without an FFI shim — the output IS C. Zig requires you to live inside the Zig toolchain.

5. **Mandatory module-header annotations.** `@owns` / `@arch` / `@depends` / `@threading` / `@invariants` are compiler-checked in fastC. Every module declares its architectural contract at the top of the file. Zig has no such convention; reviewers grep for context.

6. **Vendor-first dependencies with Sigstore (scheduled).** Zig has the Zon registry. fastC has no central registry — every dep is git URL + commit + sha256 + Sigstore bundle. There's nothing to compromise on a registry server because there is no registry server.

7. **Native MCP server.** Zig agent tools text-scrape `zig build` output. fastC ships `fastc-mcp` as a first-class agent interface — AST, types, caps, contracts served over Model Context Protocol without text parsing.

8. **NASA/JPL Power-of-10 enforcement.** `fastc check --safety-level=critical` enforces no-recursion, no-allocation, bounded-loops, function-size limits. Zig has no equivalent CLI gate.

### Where Zig wins over fastC

1. **Compile time.** Zig 149 ms vs fastC 215 ms on the four-program benchmark — Zig is ~30% faster end-to-end. fastC's planned tcc dev-mode backend (stage 0.8) targets sub-200 ms but hasn't shipped.

2. **Maturity.** Zig 0.16 is shipping with a real library ecosystem. fastC is pre-1.0 with a sub-thousand-package world.

3. **C ingestion.** Zig's `@cImport` consumes any C header automatically. fastC requires hand-written `extern "C"` blocks. The deliberate trade is auditability (every external C symbol fastC touches is enumerated and reviewable), but it's a real ergonomic loss.

4. **`comptime`.** Zig's compile-time metaprogramming is genuinely powerful — generic containers without monomorphization explosions, embedded DSLs, build-time codegen. fastC has no equivalent; we trade expressive power for predictable codegen and a smaller language surface.

5. **Cross-compilation.** Zig is the best cross-compiler in the world out of the box. fastC doesn't ship cross-targets yet.

### One-sentence positioning

Zig and fastC made opposite choices about what to push to the language level. Zig prioritized performance and metaprogramming (`comptime`, cross-compilation, C ingestion). fastC prioritized safety and auditability (capability-typed I/O, declarative manifests, mandatory contracts, MCP-native diagnostics). On the dimensions both share — binary size, no-GC native code, refusing silent UB — they're tied. The wedge is everywhere else.
