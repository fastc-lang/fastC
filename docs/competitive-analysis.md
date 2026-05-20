# Competitive Analysis

This document positions fastC against C, Zig, Rust, and V. It is honest about where fastC wins, where it must catch up, and what lessons we take from each competitor. The analysis is structured around the strategic wedge documented in [docs/roadmap.md](roadmap.md) and [docs/MANIFESTO.md](MANIFESTO.md): capability-typed I/O, zero executable build scripts, mandatory contracts, capability-aware dependencies with Sigstore/SLSA L3 provenance, and a compile-time budget enforced in CI.

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

**Supply chain:**
- No package manager and no central registry. Dependencies are header files copied into the source tree or system-installed via the OS package manager. This is the closest thing to fastC's vendor-first model in the existing systems-programming world.
- Build systems (Make, CMake, Autotools) execute arbitrary shell during the build. The supply-chain attack surface is the build script, not the package manager — which is why CI/CD compromises in the C world tend to target Makefiles and configure scripts.

**Lessons for fastC:**
- C interop is non-negotiable. fastC transpiles to C11, giving source-level interop with no FFI overhead.
- Don't try to replace C's ecosystem — integrate with it. fastC code should call C libraries directly via `extern "C"` declarations.
- C's simplicity is a feature. fastC should stay simple: no GC, no runtime beyond the small `fc_*` header, no hidden allocations.
- The C lesson on build scripts: the attack surface is the part that runs, not the part that declares. fastC eliminates the part that runs by design.

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

**Supply chain:**
- `build.zig` is an arbitrary Zig program executed during every build. This is the same attack surface Rust has with `build.rs` and worse than what fastC accepts.
- Zig's package manager (Zon, released 2024) ships content-hashed dependencies — better than Cargo's pre-cargo-vet baseline. But there is no PURL type, no Sigstore provenance, no Snyk/Socket/Dependabot integration. Per McKayla Washburn's January 2026 Zig package talk: "You can't get adoption without tooling, and you can't get tooling without adoption" — the M×N integration tax has not been paid.
- The talk's core proposal — "how about we just don't execute arbitrary code in the package manager/during builds?" — is exactly the move fastC is making.

**Lessons for fastC:**
- Zig's explicitness is the right direction. fastC follows the same principle.
- `zig fmt` with zero configuration is correct. fastC copies this: `fastc fmt` has no options.
- Don't adopt `comptime`. Monomorphization (0.9) gives generics without evaluation phases.
- Zig's C interop via built-in compiler is clever but ties you to a specific C toolchain. fastC's source-level transpilation is more portable, and avoids ingesting arbitrary C source (which is the supply-chain trade we are making).
- Take McKayla's package-talk argument and *ship* it: declarative manifests only, content-hashed deps, Sigstore signing from day one.

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

**Supply chain (this is where Rust's competitive position weakened most in 2025/2026):**
- `build.rs` and proc macros execute arbitrary Rust at compile time. The dominant attack surface in 2025–2026 incidents. Concrete cases: `faster_log` and `async_println` (2025, ~8,424 downloads before takedown, stealing Solana/Ethereum private keys via build-time code execution); `evm-units` (2025, 7,000+ downloads, OS-specific payload delivery); the rustfoundation.dev phishing campaign (September 2025, harvested GitHub credentials of crate authors); the `timeapis.io` package campaign (CVE-2026-28353, exfiltrating `.env` secrets from CI in early 2026).
- Crates.io has the best ecosystem integration of any systems-language registry (PURL, SBOM tooling, cargo-vet, cargo-audit, GitHub dependency graph) but the underlying property — packages can run arbitrary code — is structural and cannot be patched without breaking the ecosystem.
- The Bun team's published `rust-rewrite-plan.md` makes the broader argument: "the Zig→Rust delta is real: the Zig bugs are exactly the destructor/ownership-fixable kind... The proposal is to remove the largest bug class structurally rather than fix instances of it indefinitely." The same argument applied to supply chain: remove the build-script bug class structurally rather than patch instances forever.
- Zack Wong's framing from the same window: "Many of Zig's greatest features were designed for human ergonomics, but this doesn't really matter that much to agents... With coding agents allowing 100x more code to be written, this also means you need to scrutinize 100x more Zig code for memory issues." Substitute "Rust supply chain" for "Zig memory" and the conclusion is the same — structural fix, not instance-by-instance audit.

**Lessons for fastC:**
- Rust proves that safety and performance are compatible. fastC aims for the same.
- `cargo` proves that integrated tooling matters. fastC integrates `build`, `fmt`, `check`, `test`, `new`.
- Don't copy Rust's lifetime system. fastC's pointer types (`ref`, `mref`, `own`, `raw`) are simpler. Some Rust-level safety guarantees are sacrificed for usability — this is an intentional trade-off documented in the [Manifesto](MANIFESTO.md).
- Copy Rust's structured diagnostics, but make them first-class (all commands, not just `rustc`), and serve them over MCP (1.6) so agents do not have to text-parse `cargo check`.
- Reject the entire concept of executable build scripts. This is the wedge — and the reason existing Rust hardening tools (cargo-vet, cargo-audit, Snyk, Socket) are necessary in the first place.

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

| Feature | C | Zig | Rust | V | **fastC** |
|---------|---|-----|------|---|-----------|
| Memory safety | None | Runtime checks | Compile-time (borrow checker) | Claimed (incomplete) | **Runtime checks + unsafe escape** |
| C interop | Native | Built-in C compiler | FFI wrappers | Claimed | **Source-level (transpiles to C11)** |
| Build system | External (Make, CMake) | Built-in (`build.zig` executes Zig) | cargo (`build.rs` executes Rust) | Built-in | **Built-in declarative manifest only** |
| **Executable build scripts** | Make / CMake (shell) | `build.zig` (arbitrary Zig) | `build.rs` + proc macros | postinstall-style | **❌ None. Refused by design.** |
| **Package manager** | None | Zon (content hashes) | crates.io (registry account) | vpkg | **Vendor-first + git+hash (1.7)** |
| **Capability typing** | None | None | None | None | **✅ `@caps(fs.read, ...)` typed args (1.4)** |
| **Contract checking** | assert.h (runtime) | comptime asserts (manual) | `debug_assert` (manual) | None | **✅ `@requires` / `@ensures` mandatory (1.5 runtime, 2.1 SMT)** |
| **SBOM / provenance** | None | None | crates.io + cargo-vet (opt-in) | None | **✅ Sigstore + SLSA L3 mandatory (1.7)** |
| **Compile-time budget in CI** | N/A | None | None | Claimed (disputed) | **✅ `compile-time-budget.toml` CI-gated (0.8)** |
| **Agent protocol** | None | None | None (text-parse `cargo check`) | None | **✅ `fastc-mcp` MCP server (1.6)** |
| Formatter | clang-format (configurable) | zig fmt (zero config) | rustfmt (configurable) | vfmt | **fastc fmt (zero config)** |
| Generics | Macros / `_Generic` | comptime | Monomorphization + traits | Claimed | **Monomorphization (0.9)** |
| Error format | Text | Text | Text + JSON | Text | **JSON-first (1.6)** |
| Auto-fix | None | None | cargo fix (limited) | None | **fastc fix (1.6)** |
| Agent tooling | None | None | None | None | **fastc context, fastc diff, fastc explain (1.6)** |
| Learning curve | Medium | Medium | High | Claimed low | **Low (C-like with safety)** |
| Maturity | 50+ years | Pre-1.0 | Stable | Unstable | **Pre-1.0** |
| Safety certifications | DO-178C, IEC 62304 | None | None | None | **Planned (2.2, via C11 output + contracts + capabilities)** |

## Where fastC Wins

### 1. Supply-chain story (the structural win)

No other systems language refuses executable build scripts at the language design level. Rust ships `build.rs`. Zig ships `build.zig`. Both execute arbitrary code during the build. fastC's manifests are declarative — there is no place to put a payload. Dependencies are git URL + commit + content hash, vendored into the project. The compiler binary is Sigstore-signed and ships SLSA L3 provenance.

This is not "Rust with cargo-vet enabled." This is the property the entire cargo-vet / cargo-audit / Snyk / Socket ecosystem exists to *retrofit*, structurally absent from fastC by design. See [docs/supply-chain.md](supply-chain.md).

### 2. Capability-typed I/O (the wedge for AI-generated code)

Capabilities are typed function arguments, minted only in `main`. A function with no capability arguments structurally cannot do I/O — checked at compile time, not by a runtime sandbox. Compare:

- **E2B / Northflank / Modal / Microsoft Agent Governance Toolkit** — runtime sandboxing of agent-generated code. Works, but expensive, slow, and only catches what the sandbox sees.
- **Rust** — every function can call `std::fs::read` from anywhere. No structural answer to prompt injection in generated code.
- **fastC** — the agent generates a function with `@caps()` (empty). The compiler rejects any I/O. No sandbox needed.

This is Austral's design (Borretti, 2022), but with surface syntax built for LLM tokenizers and a stdlib born capability-aware. See [docs/capabilities.md](capabilities.md).

### 3. Agent-friendly diagnostic surface served over MCP

No other systems language explicitly optimizes for AI coding agents. fastC's [agent-first features](agent-features.md) — structured JSON diagnostics, `fastc fix`, `fastc context`, `fastc diff`, `fastc explain`, the `fastc-mcp` server — create a workflow where agents query the AST, capability graph, and contract discharge over a typed protocol, instead of text-parsing `cargo check` output.

As AI-assisted development becomes the norm, the language that agents produce the most correct code in *per token* will see disproportionate adoption. See [docs/mcp.md](mcp.md).

### 4. Source-level C interop

fastC transpiles to readable C11. This means:
- No FFI overhead — calling a C library is zero-cost.
- Existing C tooling (Valgrind, gdb, sanitizers) works on fastC output.
- C libraries don't need bindings — declare extern signatures, link normally.
- Safety-critical certification can leverage existing C11 qualification evidence.

Zig's C interop ingests arbitrary C source — better at consumption, but inherits whatever the C source brought in. fastC accepts the C-ingestion loss in exchange for not parsing or trusting arbitrary C.

### 5. Safety-critical certification path (made stronger by contracts + capabilities)

DO-178C and IEC 62304 require qualified compilers or evidence that the compiler does not introduce defects. fastC's approach:
- Transpile to C11 (auditable, readable output).
- Compile with a qualified C compiler (gcc or clang with qualification kits).
- The fastC transpiler only needs to be shown to produce correct C — a much easier certification argument than qualifying an entire compiler backend.
- **New in this roadmap:** contract discharge reports (`discharge.json`) and capability graphs (`caps.json`) feed auditors structural evidence that "this subsystem cannot allocate" or "this subsystem cannot reach the network" — exactly what DO-178C wants to see.

No other modern safety-first language has this combination.

### 6. Learning curve

fastC syntax is C-like with explicit safety additions. A C programmer can read fastC code immediately. The type system adds `ref`, `mref`, `own`, `opt`, `res`, `slice`, and `arr` — six concepts, not a full ownership/lifetime system.

Compare this to Rust, where a C programmer must learn ownership, borrowing, lifetimes, traits, pattern matching, and the module system before being productive.

## Where fastC Must Catch Up

### 1. Generics (0.9)

Without generics, fastC cannot express generic data structures or algorithms. This is the most critical missing language feature. Monomorphization is planned for 0.9 — until then, users must write type-specific code or use `raw` pointers with `unsafe`.

### 2. Standard library (1.1)

fastC currently has no standard library. Users must call C standard library functions via FFI. A minimal standard library (io, string, vec, hashmap, mem, math, fs, net) is planned for 1.1. It is born capability-aware in shape so 1.4 only enforces checking without rewriting the surface.

### 3. Curated ecosystem (1.8)

fastC has no packages. The vendor-first package system (1.7) lands the infrastructure; the curated `fastc-core` stdlib extensions (1.8) ship the first 5 audited packages (http, json, toml, log, cli) and grow to ~30–50 over 12 months. Until then, fastC depends on C library interop and locally vendored code. See [docs/ecosystem.md](ecosystem.md).

### 4. Distribution

Zero stars, one fork at the time of this writing. The language does not get adopted on technical merit alone. The 8-week launch plan in the roadmap exists for exactly this reason: ship the benchmark + manifesto + 5 curated packages, then post coordinated to HN, r/programming, and r/rust.

### 5. Maturity

fastC is pre-1.0. The language may change. The compiler has not been fuzzed (stage 2.0). There is no formal specification yet (stage 1.1). Production use is not recommended until 1.0 ships traits + stable language surface.

This is stated honestly, not hidden. See V's mistakes above.

## Strategic Priorities

The post-0.6 stage ordering encodes a thesis: ship the structural wins before the cosmetic ones, and prove every claim with a measurable artifact.

1. **0.7–0.8: Compile-time discipline first.** Modules land (0.7, mostly done) and the compile-time budget gate (0.8) lands *before* stdlib, so no future feature can quietly bloat compile times. This is what every "safer C" predecessor failed at.
2. **0.9–1.0: Generics + traits.** Without these, real code does not exist. Land them under the budget gate so the cost is visible.
3. **1.1–1.2: Stdlib + the launch benchmark.** Stdlib (1.1) and benchmarks (1.2) — including the token-efficiency benchmark with Claude/GPT/Gemini on fastC vs Rust vs Zig vs Go — are the launch artifacts. The 1.2 numbers are what go on Hacker News.
4. **1.3–1.5: The annotation surface.** Annotation grammar + module headers (1.3), capabilities (1.4), and runtime-tier contracts (1.5) deliver the wedge. After 1.5, every public function in fastC is a typed operating manual.
5. **1.6: Agent + MCP.** `fastc-mcp` exposes everything (1.3 + 1.4 + 1.5 + existing diagnostics) over Model Context Protocol. Native protocol for Claude Code, Cursor, Codex.
6. **1.7–1.8: The supply-chain story.** Vendor-first + Sigstore + SLSA L3 (1.7), and the first 5 curated `fastc-core` packages (1.8). The structural rebuttal to crates.io / build.rs / proc macros.
7. **2.0–2.3: Hardening, SMT, certification, async.** Compiler hardening (2.0), SMT contract discharge (2.1), DO-178C / IEC 62304 evidence packaging (2.2), async via Future trait (2.3).

The key insight: fastC does not need to beat Rust at safety or Zig at explicitness. It needs to be **good enough** at both while being **the only** language that combines capability-typed I/O, zero executable build scripts, mandatory contracts, capability-aware deps with Sigstore/SLSA L3 provenance, and a CI-enforced compile-time budget. That's a defensible position no competitor is pursuing — and one Rust and Zig cannot retrofit without breaking their ecosystems.
