# Competitive Analysis

This document positions FastC against C, Zig, Rust, and V. It is honest about where FastC wins, where it must catch up, and what lessons we take from each competitor.

## Per-Competitor Analysis

### C

**The incumbent.** 50+ years of ecosystem, runs everywhere, understood by every systems programmer.

**Strengths:**
- Universal availability — every platform has a C compiler.
- Minimal runtime overhead — what you write is what runs.
- Massive ecosystem of libraries, tools, and expertise.
- Stable ABI — libraries compiled decades ago still link.
- Safety-critical certifications (DO-178C, IEC 62304) are well-established for C.

**Weaknesses:**
- No memory safety — buffer overflows, use-after-free, null pointer dereferences.
- Undefined behavior silently produces wrong results.
- Ambiguous grammar (declaration vs. expression, "most vexing parse").
- Error messages from gcc/clang are text-only with no machine-readable format.
- No standard build system, package manager, or formatter.
- Macro-heavy patterns are fragile and hard for agents to generate.

**Lessons for FastC:**
- C interop is non-negotiable. FastC transpiles to C11, giving source-level interop with no FFI overhead.
- Don't try to replace C's ecosystem — integrate with it. FastC code should call C libraries directly.
- C's simplicity is a feature. FastC should stay simple: no GC, no runtime, no hidden allocations.

### Zig

**The modern C replacement.** Explicit, no hidden control flow, comptime metaprogramming.

**Strengths:**
- Extremely explicit — no hidden allocations, no hidden control flow.
- `comptime` is powerful and replaces macros, generics, and conditional compilation.
- Built-in allocator parameter pattern encourages explicit memory management.
- Ships with its own C compiler (integrated C interop).
- Good documentation and error messages.
- `zig fmt` enforces a single style.

**Weaknesses:**
- `comptime` is a paradigm shift that agents struggle with — it requires understanding evaluation phases.
- Still pre-1.0, with breaking changes.
- Ecosystem is small compared to C or Rust.
- No structured diagnostic output (text only).
- The allocator pattern adds verbosity to every data structure interaction.

**Lessons for FastC:**
- Zig's explicitness is the right direction. FastC follows the same principle.
- `zig fmt` with zero configuration is correct. FastC copies this: `fastc fmt` has no options.
- Don't adopt `comptime`. Monomorphization (0.8) gives generics without evaluation phases.
- Zig's C interop via built-in compiler is clever but ties you to a specific C toolchain. FastC's source-level transpilation is more portable.

### Rust

**The safety-first language.** Ownership, lifetimes, and the borrow checker prevent entire classes of bugs at compile time.

**Strengths:**
- Ownership and borrowing prevent use-after-free, data races, and dangling pointers at compile time.
- `cargo` is the gold standard for build tools and package management.
- Rich type system with traits, generics, and pattern matching.
- Excellent error messages with suggestions.
- Strong ecosystem (crates.io has 150K+ crates).
- `cargo fix` provides some auto-fix capability.

**Weaknesses:**
- Steep learning curve — lifetimes, trait bounds, and the borrow checker are hard.
- Compile times are slow for large projects.
- Macro system (`proc_macro`, `macro_rules!`) produces code that's hard for agents to generate and debug.
- `unsafe` Rust is harder to audit than C because of the complex invariants.
- Binary size is large due to monomorphization and the standard library.
- C interop requires FFI wrappers (`extern "C"`, `bindgen`, `cbindgen`).
- `--error-format=json` exists but isn't used by most tooling.

**Lessons for FastC:**
- Rust proves that safety and performance are compatible. FastC aims for the same.
- `cargo` proves that integrated tooling matters. FastC integrates `build`, `fmt`, `check`, `test`, `new`.
- Don't copy Rust's lifetime system. FastC's pointer types (`ref`, `mref`, `own`, `raw`) are simpler. Some Rust-level safety guarantees are sacrificed for usability — this is an intentional trade-off.
- Copy Rust's structured diagnostics, but make them first-class (all commands, not just `rustc`).

### V

**The cautionary tale.** Promised "the simplicity of Go with the performance of C" but overpromised and underdelivered.

**Strengths:**
- Simple syntax, fast compilation.
- Ambitious vision for a modern systems language.

**Weaknesses:**
- Claimed features as "done" when they were incomplete or broken (autofree, generics, standard library).
- Promised 400x faster than GCC compilation — benchmark methodology was flawed.
- Memory management story changed multiple times (GC → autofree → manual).
- Community trust eroded due to vaporware claims.

**Lessons for FastC:**
- **Honesty is non-negotiable.** This roadmap only marks features `[x]` when they pass tests. The [benchmarking methodology](benchmarking.md) documents anti-patterns explicitly.
- **Don't claim what you haven't built.** Every milestone has a Definition of Done with concrete, verifiable criteria.
- **Acknowledge trade-offs.** FastC's transpilation has a real compile-time cost. Bounds checks have a real runtime cost. We measure and report both honestly.
- **Underpromise, overdeliver.** Better to ship late than to ship broken.

## Positioning Matrix

| Feature | C | Zig | Rust | V | **FastC** |
|---------|---|-----|------|---|-----------|
| Memory safety | None | Runtime checks | Compile-time (borrow checker) | Claimed (incomplete) | **Runtime checks + unsafe escape** |
| C interop | Native | Built-in C compiler | FFI wrappers | Claimed | **Source-level (transpiles to C11)** |
| Build system | External (Make, CMake) | Built-in | cargo | Built-in | **Built-in (fastc build)** |
| Package manager | None | Gyro (community) | crates.io | vpkg | **Planned (1.5)** |
| Formatter | clang-format (configurable) | zig fmt (zero config) | rustfmt (configurable) | vfmt | **fastc fmt (zero config)** |
| Generics | Macros / `_Generic` | comptime | Monomorphization + traits | Claimed | **Monomorphization (0.8)** |
| Error format | Text | Text | Text + JSON | Text | **JSON-first (1.2)** |
| Auto-fix | None | None | cargo fix (limited) | None | **fastc fix (1.2)** |
| Agent tooling | None | None | None | None | **fastc context, fastc diff (1.2)** |
| Learning curve | Medium | Medium | High | Claimed low | **Low (C-like with safety)** |
| Maturity | 50+ years | Pre-1.0 | Stable | Unstable | **Pre-1.0** |
| Safety certifications | DO-178C, IEC 62304 | None | None | None | **Planned (2.0+, via C11 output)** |

## Where FastC Wins

### 1. Agent Usability

No other systems language explicitly optimizes for AI coding agents. FastC's [agent-first features](agent-features.md) — structured JSON diagnostics, `fastc fix`, `fastc context`, `fastc diff`, inline tests — create a workflow where agents can iterate to working code without human intervention.

This is not a theoretical advantage. As AI-assisted development becomes the norm, the language that agents produce the most correct code in will see disproportionate adoption.

### 2. Source-Level C Interop

FastC transpiles to readable C11. This means:
- No FFI overhead — calling a C library is zero-cost.
- Existing C tooling (Valgrind, gdb, sanitizers) works on FastC output.
- C libraries don't need bindings — just `#include` the header.
- Safety-critical certification can leverage existing C11 qualification evidence.

Zig's C interop is close but ties you to Zig's bundled clang. Rust requires `bindgen`/`cbindgen` and `extern "C"` wrappers.

### 3. Safety-Critical Certification Path

DO-178C and IEC 62304 require qualified compilers or evidence that the compiler doesn't introduce defects. FastC's approach:
- Transpile to C11 (auditable, readable output).
- Compile with a qualified C compiler (gcc or clang with qualification kits).
- The FastC transpiler only needs to be shown to produce correct C — a much easier certification argument than qualifying an entire compiler backend.

No other modern safety-first language has this certification path.

### 4. Learning Curve

FastC syntax is C-like with explicit safety additions. A C programmer can read FastC code immediately. The type system adds `ref`, `mref`, `own`, `opt`, `res`, `slice`, and `arr` — six concepts, not a full ownership/lifetime system.

Compare this to Rust, where a C programmer must learn ownership, borrowing, lifetimes, traits, pattern matching, and the module system before being productive.

## Where FastC Must Catch Up

### 1. Generics (0.8)

Without generics, FastC cannot express generic data structures or algorithms. This is the most critical missing feature. Monomorphization is planned for 0.8 — until then, users must write type-specific code or use `raw` pointers with `unsafe`.

### 2. Standard Library (1.0)

FastC currently has no standard library. Users must call C standard library functions via FFI. A minimal standard library (io, string, vec, hashmap, mem, math, fs) is planned for 1.0.

### 3. Ecosystem (1.5)

FastC has no packages, no registry, no community libraries. The package registry is planned for 1.5, with 10-20 seed packages. Until then, FastC depends entirely on C library interop.

### 4. Maturity

FastC is pre-1.0. The language may change. The compiler has not been fuzzed. There is no formal specification. Production use is not recommended until 1.0.

This is stated honestly, not hidden. See V's mistakes above.

## Strategic Priorities

1. **0.7–0.8**: Close the feature gap (modules, generics). Without these, FastC cannot be taken seriously.
2. **0.9–1.0**: Reach MVP (traits, standard library). This is the minimum for non-trivial programs.
3. **1.1–1.2**: Differentiate (benchmarks, agent features). This is where FastC becomes uniquely valuable.
4. **1.5+**: Build ecosystem (packages, community). This is where adoption happens.

The key insight: FastC doesn't need to beat Rust at safety or Zig at explicitness. It needs to be **good enough** at both while being **the best** at agent usability and C interop. That's a defensible position that no competitor is pursuing.
