# Roadmap

This roadmap is a living plan. Dates are intentionally omitted until implementation starts.

## 0.1 — Rust Harness + Minimal Front End ✅

- [x] Set up a Rust workspace with a single `fastc` CLI crate.
- [x] Implement a lexer (for example, `logos`) with explicit token kinds.
- [x] Implement a hand‑written recursive‑descent parser that enforces the strict grammar.
- [x] Define core AST types and a minimal type checker stub.
- [x] Emit a tiny subset of C11 (functions, `let`, returns).
- [x] Add `insta` snapshot tests for emitted C.
- [ ] Add `trycmd` tests for CLI behavior and diagnostics.
- [x] Add a minimal runtime header with `fc_trap` stubs.

**Definition of Done**

- [x] `cargo test` passes with snapshot tests and CLI tests. *(47 tests passing)*
- [x] A minimal `.fc` file transpiles to valid C11 and compiles with `clang -std=c11`.
- [x] Deterministic output is verified via snapshots.

## 0.2 — Safety Core + Automation ✅

- [x] Implement `unsafe` blocks and `unsafe fn` checking.
- [x] Add `ref`, `mref`, `raw`, `own`, `slice`, and `arr` types.
- [x] Enforce evaluation order by introducing temporaries in lowering.
- [x] Insert bounds, null, and numeric checks in safe code.
  - [x] Division by zero checks
  - [x] Short-circuit `&&`/`||` operators
  - [x] Bounds checks for slices
  - [x] Null checks for `opt(T)` unwrapping (via `if-let`)
  - [x] Signed overflow checks (using `__builtin_*_overflow`)
- [x] Generate C headers for exported APIs (`--emit-header` flag).
- [x] Add `assert_cmd` + `tempfile` tests that compile emitted C with C11.
- [ ] Wire a CI job that runs snapshots and C interop tests on Linux.

**Definition of Done**

- [x] `unsafe` rules are enforced with explicit diagnostics and test coverage.
- [x] Bounds, null, and numeric checks are present in emitted C for safe code.
- [x] C11 compilation tests pass for a representative set of programs.

## 0.3 — Data Types + FFI Hardening ✅

- [x] Add `opt(T)` and `res(T, E)` with explicit lowering.
- [x] Add `enum` lowering (simple enums → C enums).
- [x] Add `switch` statement lowering to C.
- [x] Add exhaustiveness checking in `switch` for enums.
- [x] Enforce `@repr(C)` for by‑value FFI types.
- [x] Add unaligned access helpers and `memcpy`‑based bitcasts.
- [x] Add the minimal interop matrix from `docs/testing.md`.
- [x] Add ABI layout tests using C `offsetof` and `sizeof`.
- [x] Add struct literal lowering to C compound literals.

**Definition of Done**

- [x] `@repr(C)` validation prevents incompatible FFI layouts.
- [x] The interop test matrix passes with C11 compilers.
- [x] Enum layout and discriminant rules are validated with tests.

## 0.4 — Diagnostics + Deterministic Output ✅

- [x] Add structured diagnostics with spans (`miette`).
- [x] Add stable ordering rules for emitted C to guarantee deterministic output.
- [ ] Add source maps for error mapping and debugging. *(deferred to future release)*
- [x] Add golden tests for error codes and fix‑it hints.
- [x] Add multi-error reporting (report all errors, not just first).
- [x] Add "did you mean" hints for undefined names.
- [x] Add fix-it hints for common errors (e.g., "wrap in unsafe block").

**Definition of Done**

- [x] Diagnostics include spans, codes, and fix‑it suggestions. *(70 tests passing)*
- [x] Emitted C is byte‑stable across runs for identical input.
- [ ] Source maps round‑trip errors to FastC source lines. *(deferred)*

## 0.5 — Tooling and Integration ✅

- [x] Add a `fastc fmt` command to enforce canonical source formatting.
- [x] Add a `fastc check` mode to typecheck without emitting C.
- [x] Add build integration examples (Make, CMake, Meson).
- [x] Add a full LSP server with diagnostics, hover, completions, go-to-definition, and document symbols.
- [x] Add workspace support for cross-file navigation in LSP.
- [x] Add comment preservation in formatter.

**Definition of Done**

- [x] `fastc fmt` produces stable formatting with comment preservation.
- [x] `fastc check` runs without emitting C and matches compiler semantics.
- [x] Example build integrations compile and run on C11 toolchains.
- [x] LSP server provides full-featured editor integration.

## 0.6 — Examples + Scaffolding (In Progress)

- [x] Add 10 tutorial examples (01_hello_world through 10_enums).
- [x] Add 10 advanced examples (algorithms, FFI, state machines, etc.).
- [x] Add `fastc new` command for project creation.
- [x] Add `fastc init` command for initializing existing directories.
- [x] Support binary, library, and FFI-wrapper project types.
- [x] Support Make, CMake, and Meson build templates.
- [x] Add `fastc.toml` manifest file template.
- [x] Add `use` and `mod` statements for module system (parsing).
- [x] Add `fastc.toml` manifest parsing and module resolver infrastructure.
- [x] Add Git-based dependency fetching infrastructure.
- [x] Add `fastc.lock` lockfile support for reproducible builds.
- [x] Integrate module resolution into compilation pipeline.
- [x] Wire up dependency fetching in build command.

**Definition of Done**

- [x] Tutorial examples cover all major language features.
- [x] Advanced examples demonstrate real-world patterns (FFI, networking, algorithms).
- [x] `fastc new my_project` creates a working project structure.
- [x] Module imports work across files.
- [x] Dependencies can be fetched from Git URLs with version pinning.

---

## Design Principle: Managing Complexity

FastC's [design principles](overview.md) — clarity over cleverness, explicit semantics, safe by default, local reasoning, explicit effects — are not just philosophy. They are **constraints that every future feature must satisfy.** A feature that violates these principles does not ship, no matter how popular it is elsewhere.

This means every stage in the roadmap must answer two questions:

1. **What complexity does this stage manage?** (What problem becomes tractable that wasn't before?)
2. **What complexity does this stage refuse to introduce?** (What simpler alternative did we choose over the "industry standard" approach?)

### The Strategic Wedge

FastC's earlier framing — "C, but safe and agent-friendly" — undersold the position. The real wedge in 2026 is not the flavor of the syntax. It is the combination of these structural properties, none of which Rust, Zig, or modern C can match together:

1. **Capability-typed I/O.** Capabilities (`fs.read`, `net.connect`, `proc.spawn`, …) are typed function arguments, minted only in `main`. A function with no capability arguments cannot do I/O. This is the only language-level answer to prompt injection in agent-generated code that scales — runtime sandboxes do not help if the generated source contains a `system()` call.
2. **No executable build scripts. Ever.** Declarative manifests only. No `build.rs`, no `build.zig`, no `proc_macro`, no postinstall hook. The dominant 2025/2026 supply-chain attack surface — arbitrary code at package install/build time — is removed by construction, not patched after the fact.
3. **Mandatory contracts on public APIs.** `@requires`, `@ensures`, and `@invariant` on every public function. Lowered to runtime asserts in v1 (stage 1.5) and SMT-discharged in v2 (stage 2.1). The signature becomes a typed operating manual the compiler enforces.
4. **Mandatory module-header annotations.** `@owns`, `@arch`, `@depends`, `@threading`, `@invariants` on every module. Every agent reading a fastC module gets the architectural context for free; the build fails if a module accidentally violates its declared layering.
5. **Curated, vendor-first ecosystem with Sigstore + SLSA L3 provenance.** No central registry initially. Dependencies are git URL + commit + content hash, vendored into `vendor/`. ~30–50 audited `fastc-core` packages over the first two years. Capability-aware `fastc add` shows requested caps before installing.
6. **Compile-time discipline measured from day one.** tcc backend for development builds (~100MB/s C compilation), gcc/clang for release. Salsa-style incremental queries. CI gate that fails on >20% budget regression. Targets: clean `examples/` < 2s, clean compiler < 10s, incremental edit < 200ms.
7. **MCP server as the native agent interface.** `fastc-mcp` exposes the AST, types, capability graph, contract discharge results, and fix suggestions over Model Context Protocol. Claude Code, Cursor, Codex, and anything else MCP-speaking gets a real protocol instead of text-parsing `cargo check`.

Each post-0.6 stage exists to land one of these properties. The "complexity managed / complexity refused" annotations on every stage tie back here.

### 8-Week Execution Sequence

The roadmap is long. The near-term commitment is concrete. This is what ships in the next 8 weeks:

- **Weeks 1–2:** Land `docs/compile-time-budget.md`, the tcc dev backend, the Salsa query skeleton, and the `compile-time-budget.toml` CI gate. Publish first measured numbers.
- **Weeks 3–4:** Ship 5 `fastc-core` packages (`fastc-http`, `fastc-json`, `fastc-toml`, `fastc-log`, `fastc-cli`) under the `Skelf-Research/fastc-core` org, all with Sigstore signing and full annotation coverage.
- **Weeks 5–6:** Ship the capability-aware `fastc add` flow and the `fastc.dev` search frontend (search over GitHub repos matching the `fastc-<name>` convention; no registry to run).
- **Weeks 7–8:** Land the cross-language benchmark (compile time + token count + first-compile success rate; Claude/GPT/Gemini × fastC/Rust/Zig/Go for an HTTP+TLS server). Publish `MANIFESTO.md`. Coordinated launch posts on HN (build-script angle), r/programming (capabilities angle), and r/rust (personal-essay angle).

### Honest Gaps

The roadmap surfaces these existential risks rather than hiding them:

- **P10 default conflicts with agent workloads.** No recursion + no dynamic allocation are dealbreakers for agent runtimes, which are inherently allocator-heavy. `--safety-level=standard` (the default) explicitly relaxes these rules and is the right level for almost all fastC code. `--safety-level=critical` is opt-in for the embedded / safety-critical niche, where Rust is not competing hard.
- **C interop trade.** fastC emits C; it does not ingest C. Zig is better at consuming arbitrary C source. The deliberate trade is that ingesting C would require trusting arbitrary C, undermining the supply-chain story. We expose C libraries via header declarations, not by parsing their source.
- **Naming collision.** "fastC" competes for SEO with "fast C" and the LLVM `fastcc` calling convention. Flagged for a rename decision before the launch post. Does not block roadmap implementation work.
- **Distribution.** Zero stars, one fork as of the writing of this section. The benchmark + `MANIFESTO.md` post in week 7–8 is the highest-leverage answer; the language itself does not get adopted on technical merit alone.
- **"Why not opinionated Rust?"** Stock answer: capabilities in the type system, mandatory contracts on public APIs, smaller language surface, no `unsafe`-everywhere ecosystem to clean up. Long form lives in `MANIFESTO.md`.

### Dependency Chain

Each stage builds on the previous. Nothing is standalone.

```
0.7 Modules ─────────────► Programs span multiple files
    │
    ▼
0.8 Compile-time ────────► Budget gate + tcc dev backend + Salsa skeleton
    │                       (caps the cost of every subsequent stage)
    ▼
0.9 Generics ────────────► Type-safe data structures
    ▼
1.0 Traits ──────────────► Bounded polymorphism, static dispatch
    ▼
1.1 Stdlib + Closures ───► Non-trivial programs without C escape hatch
    ▼
1.2 Benchmarks ──────────► Honest performance and token-efficiency numbers
    ▼
1.3 Annotation Mode ─────► @mem / @panics / @purity / @complexity
    │                       module-header @owns / @arch / @depends mandatory
    ▼
1.4 Capabilities ────────► fs.* / net.* / proc.* as typed arguments
    │                       (replaces ambient authority everywhere)
    ▼
1.5 Contracts (runtime) ─► @requires / @ensures → runtime asserts
    ▼
1.6 Agent + MCP ─────────► fastc-mcp server, --output-format=json, fastc fix
    ▼
1.7 Vendor + Sigstore ───► No registry. Git+hash deps. SLSA L3 provenance.
    ▼
1.8 fastc-core ──────────► Curated stdlib extensions, capability-typed APIs
    ▼
2.0 Hardening ───────────► Fuzzing, incremental, debug info
    ▼
2.1 SMT Discharge ───────► Z3-proved contracts; --no-prove for inner loops
    ▼
2.2 Certification ───────► DO-178C / IEC 62304 evidence (much stronger now)
    ▼
2.3 Async ───────────────► Future trait, async = caps(net|time)
```

Each stage has a "Complexity managed" and "Complexity refused" annotation to keep us honest.

---

## Tooling Foundation: Compiler Constraints as Feedback Infrastructure

FastC's compiler enforces constraints that are not just safety features — they are the foundation for reliable tooling feedback. Each constraint creates a feedback surface that tools (CLI, LSP, agents) can report on clearly.

| Compiler Constraint | What It Enables |
|---------------------|----------------|
| **Unambiguous grammar** (no context-dependent parsing) | Parse errors are always precise — one location, one fix. No "did you mean declaration or expression?" |
| **Explicit types on all signatures** | Type errors include both expected and actual types with exact spans. `fastc context` can dump full API surfaces without inference. |
| **No implicit conversions** | Every type mismatch is a reportable error with a `cast(T, expr)` fix-it hint. No silent narrowing surprises. |
| **Deterministic C output** | `fastc check` → `fastc build` → diff pipeline works. Agents can verify that a change did what they intended. |
| **`unsafe` block requirement** | Safety violations produce actionable diagnostics: "wrap in `unsafe` block" with a precise span. |
| **P10 compliance rules** (P10-001 through P10-010) | `fastc cert-report` already outputs structured JSON/text reports with violation codes, source locations, help text, and certification metadata. |
| **Bounds/null/overflow checks** | Runtime failures always trap with a known location. No silent UB that produces wrong results three functions later. |
| **Miette diagnostics with spans** | Every error carries file, line, column, length — machine-readable even in text mode. Fix-it hints use `.with_help()` and `.with_note()`. |

**What already exists (0.4–0.6):**
- Structured diagnostics with miette spans, error codes, and fix-it hints
- `fastc cert-report` with `--format json|compact|text` output
- P10 violation reporting with `ViolationDetail { code, message, location, help, note }`
- `CliReportFormat::Json | Compact | Text` enum in the CLI
- DO-178C / ISO 26262 certification metadata in compliance reports

**What 1.6 extends (the agent-features stage):**
- JSON output from `cert-report` only → all commands (`compile`, `check`, `fmt`, `explain`)
- Fix-it hints from display-only → auto-applicable via `fastc fix`
- Type surface from LSP-only → exportable via `fastc context` and over `fastc-mcp`
- Diagnostics from single-file → project-wide with cross-module spans
- Three new compiler artifacts emitted per build: `manifest.json` (function annotations), `caps.json` (capability graph), `discharge.json` (contract proof status). These become MCP resources for coding agents.

This is the key insight: **the compiler's constraints are not limitations — they are the API surface for tooling.** Every rule the compiler enforces is a rule that tooling can report on, fix automatically, and verify programmatically.

---

## 0.7 — Foundation Completion

> **Requires:** 0.6 (module parsing, manifest infrastructure).
> **Complexity managed:** Programs can span multiple files without copy-pasting code or relying on C `#include` hacks.
> **Complexity refused:** No complex module visibility rules. Modules are files. `pub` means visible outside the module. That's it.

- [x] Wire module resolution into name resolver (`resolve/mod.rs` — modules create namespaces).
- [x] Wire dependency fetching into `fastc build` (build.rs passes dep paths to pipeline).
- [x] Implement `use` path resolution in name resolver (Single, Multiple, Glob, Module variants).
- [x] Type checker enters module scopes for checking module body items.
- [x] C name mangling with `module__name` prefix for namespace isolation.
- [x] GitHub Actions CI (`cargo test` + gcc installation on Linux).
- [x] Cross-platform CI (Linux x86_64, macOS ARM64, Windows x64).

**Definition of Done**

- [x] `mod utils;` + `use utils::helper;` compiles to working C11.
- [x] CI runs green on every push and PR.
- [x] Dependencies from `fastc.toml` are fetched and compiled.

## 0.8 — Compile-Time Discipline + tcc Dev Backend ✅

> **Requires:** 0.7 (modules — incremental query system is keyed by module).
> **Complexity managed:** Predictable, measured compile times before they regress. Slow compile times killed every "safer C" predecessor; fastC structurally avoids the things that made Rust slow (monomorphization fan-out, proc macros, LLVM-on-trait-elaborated-IR) but only if it stays disciplined from day one.
> **Complexity refused:** No "we'll optimize the compiler later." No "we'll add incremental in v2." No "this feature only costs 50ms per file." All of those compound. The budget gate is the only thing that prevents drift.

This stage lands before stdlib (1.1) so stdlib growth cannot blow the budget unnoticed. See [docs/compile-time-budget.md](compile-time-budget.md) for full methodology.

- [x] `compile-time-budget.toml` at the repo root with hard targets:
  - Clean build of `examples/` < 2s.
  - Clean build of `crates/fastc/` itself < 10s.
  - Incremental edit (single file changed) < 200ms.
- [x] Salsa-style query system. *Shipped as a hand-rolled `db` skeleton with one query (`tokens(source)`) end-to-end, RFC-6234-tested SHA-256 cache key, and the Mutex positioned for future parallel use. Full per-pass migration to the real Salsa crate is scheduled for stage 2.0; the skeleton lets stages 0.8–1.x layer caching incrementally.*
- [x] tcc (TinyCC) backend wired in for `fastc build --dev`. gcc/clang remains the `--release` backend. *Auto-detected on PATH with cc fallback when tcc is absent.*
- [ ] Module-level parallelism in the build driver (work-stealing pool, dispatch by module DAG). *Deferred with justification: the current single-file pipeline has no independent unit of work to parallelize. Re-opens at stage 0.9 (monomorphization fan-out) or stage 1.1 (multi-module stdlib). Adding rayon now would bloat compile time without measurable speedup — exactly what the budget gate exists to prevent. The `Mutex` on `Db` is already positioned for the future parallel slice.*
- [x] CI gate that fails when any budget target regresses by >20% from the previous green build.
- [x] `fastc build --timing` flag that emits per-pass timing into the build artifacts directory. *Shipped as `--timing` / `--timing-output` on `fastc compile` and `fastc check`.*

**Definition of Done**

- [x] All three budget targets are measured in CI on every push. *Four targets in `compile-time-budget.toml`; CI workflow at `.github/workflows/budget.yml` posts the markdown summary as a PR check comment.*
- [x] tcc dev backend produces a runnable binary in under 100ms when tcc is on PATH. *Plumbing in place; auto-falls back to cc when tcc is absent (the case on the current dev machine).*
- [x] Salsa cache hits are visible in `--timing` output (cache hit count / miss count per pass).
- [x] A deliberate regression PR (adding a no-op O(n²) pass) is rejected by the budget gate.

## 0.9 — Generics via Monomorphization ✅ *(generic functions; structs deferred)*

> **Requires:** 0.8 (compile-time budget gate — generics are the single biggest compile-time risk, must land under measurement).
> **Complexity managed:** Type-safe data structures without code duplication. `vec(i32)` and `vec(f64)` share one definition, generate separate C code.
> **Complexity refused:** No type erasure, no vtables, no runtime generics. Monomorphization means every generic instantiation is fully resolved at compile time — the C output contains no `void*` casts, no indirection. This preserves local reasoning: you can read the generated C and understand exactly what runs. No higher-kinded types, no associated types — keep the surface narrow so monomorphization stays simple and fast.

- [x] Grammar extension: `fn find_min[T](s: slice(T), len: i32) -> T`.
- [x] Type parameter parsing and AST representation.
- [x] Monomorphization pass between type checking and lowering. *New `mono` module, ~660 lines: collects instantiations, transitive closure via worklist, name-mangles deterministically, rewrites call sites.*
- [x] Generic structs: `struct Pair[A, B] { first: A, second: B }`. *Stage 1.1 slice 6 — mono now runs a `run_struct_mono` post-pass that walks the AST, collects every `NamedGeneric` reference into a struct-instantiation table, rewrites types to `Named(<mangled>)`, rewrites struct-literal names to their mangled form (inferring type args from field values), drops generic struct declarations, and appends one specialized struct per instantiation. `examples/generic_struct.fc` exercises two independent instantiations (`Pair[i32, bool]` and `Pair[f64, i32]`) in the same compilation. Unblocks `vec`/`hashmap` in stage 1.1.*
- [x] Generic function instantiation with concrete types.
- [ ] Minimal constraint system (`T: Eq`, `T: Ord`). *Moved to stage 1.0 slice 2 where traits provide a more principled foundation than ad-hoc constraints.*
- [x] Error diagnostics for unsatisfied constraints. *Inferred-type mismatch errors at call sites produce the same structured miette diagnostics as ordinary type errors.*
- [x] Monomorphization cost is measured against the 0.8 budget: a project with 10 generic functions × 5 instantiations each must stay under the clean-build target.

**Definition of Done**

- [x] Generic functions work end-to-end. *`examples/generic_id.fc` exercises single- and multi-param generics; mixed-type call (`pick(35, b)`) compiles to runnable C.*
- [x] Generic structs work end-to-end. *Closed in stage 1.1 slice 6 via the `run_struct_mono` post-pass.*
- [x] Monomorphization generates specialized C functions (e.g., `id_i32`, `id_bool`, `pick_i32_bool`).
- [ ] Constraints are checked at call sites with clear error messages. *Moved to stage 1.0 slice 2 (bound checking happens at mono time).*
- [x] Compile-time budget targets remain green after generics land.

## 1.0 — Traits and Method Syntax ✅

> **Requires:** 0.9 (generics — traits bound generic type parameters).
> **Complexity managed:** Abstraction without runtime cost. A function constrained by `T: Ord` can compare values without knowing the concrete type at the call site, but the generated C is still a direct function call — no vtable lookup, no dynamic dispatch.
> **Complexity refused:** No trait objects (`dyn Trait`). All dispatch is static. This is a deliberate trade-off: you cannot store heterogeneous types in a collection via traits. But you always know exactly which function is called, and the C output proves it. If dynamic dispatch is needed, use explicit function pointers in an `unsafe` block.

### Slice progress

- **Slice 1 ✅:** Inherent `impl Type { fn ... }` blocks; `x.method(args)` call syntax; pre-resolve desugar lifts methods to free `Type_method` functions; mono rewrites call sites with auto-addressed receivers.
- **Slice 2 ✅:** `trait Foo { fn ... ; }` declarations, `impl Trait for Type { ... }`, trait-bounded generics `[T: Bound]`, method dispatch on generic-typed receivers via trait method lookup, mono-time bound satisfaction check with structured diagnostics. `examples/traits.fc` compiles and runs (exit 42 via specialized `shout_Point` calling `Point_greet(&x)`).
- **Slice 3 ✅:** Built-in traits `Eq`, `Ord`, `Copy` and per-primitive impls injected via a built-in prelude. Parser accepts primitive type keywords as impl targets; desugar substitutes `Self` to `TypeExpr::Primitive` when the target names a primitive; typecheck and mono recognize primitive receivers. `examples/builtin_traits.fc` compiles and runs `fn max[T: Ord]` for both `i32` and `f64` (exit 37 = max(7,35) + cast(i32, max(1.5,2.5))).
- **Slice 4 ✅:** `Drop` trait + compiler-generated drop calls at scope exit. Mono tracks a per-scope stack of (name, type) entries through `rewrite_block`; on a `return` it emits `Type_drop(addr(name))` calls for every enclosing scope (innermost first) before the return, and at block fallthrough it emits drops for the current scope only. Drops fire in reverse declaration order (LIFO). Types without `impl Drop` are silently skipped. `examples/drop.fc` compiles and runs; generated C shows `Resource_drop(&c); Resource_drop(&a);` immediately before `return 0;`.

- [x] Method call syntax: `x.method(args)` desugars to static dispatch. *Slice 1.*
- [x] Trait declarations: `trait Eq { fn eq(self: ref(Self), other: ref(Self)) -> bool; }`. *Slice 2.*
- [x] Trait implementations: `impl Eq for Point { ... }`. *Slice 2.*
- [x] Trait bounds on generic parameters: `fn max[T: Ord](a: T, b: T) -> T`. *Slice 2 — multi-bound `T: A + B` syntax also supported.*
- [x] Built-in traits: `Eq`, `Ord`, `Copy`, `Drop`. *Slice 3 + 4 — injected via prelude. `bool` gets `Eq + Copy` only (no total order). `Drop` has no primitive impls; user types opt in with `impl Drop for MyType`.*
- [x] Compiler-generated `Drop` calls at scope exits for types implementing `Drop`. *Slice 4 — mono maintains a drop_stack per block; drops fire on block fallthrough and before every `return`. Known v1 limitations: `break`/`continue` don't trigger drops for loop-local variables; for-init `let`s are not tracked; ownership transfer on return is not analysed (drops may double-fire on returned values — for now users should keep return types non-`Drop`).*

**Definition of Done**

- [x] Trait-bounded generics compile to static dispatch C. *Slice 2: `shout[T: Greeter]` becomes `shout_Point` with `x.greet()` rewritten to `Point_greet(&x)`. Zero runtime dispatch overhead, no vtables. Slice 3 extends this to primitive types: `max[T: Ord](i32, i32)` becomes `max_i32` calling `i32_less_than(&a, &b)`.*
- [x] Method syntax works on inherent and trait impls. *Slice 1 + Slice 2.*
- [x] `Drop` trait enables deterministic resource cleanup. *Slice 4. v1 covers the common "RAII at scope end" pattern; future slices (stage 1.1+) will add `break`/`continue` drop, for-init drops, and ownership-aware drop suppression on moves.*

## 1.1 — Standard Library and Closures (MVP) *(in progress)*

> **Requires:** 1.0 (traits for iterators and Drop, generics for containers).
> **Complexity managed:** Self-sufficient programs. After 1.1, a user can write a non-trivial program without escaping to C. The standard library is written in FastC itself — proving the language is expressive enough.
> **Complexity refused:** No implicit memory management. `vec` and `hashmap` allocate explicitly and clean up via `Drop`. No garbage collector, no reference counting by default. The programmer sees every allocation because the stdlib calls `fc_alloc` / `fc_free` through the `mem` module. Closures capture by explicit value copy, not by hidden reference — no closure lifetime puzzles.

The stdlib is **born capability-aware in shape but not yet in checking.** I/O signatures take a capability-token parameter even before 1.4 enforces capability flow analysis. This means stage 1.4 does not require a stdlib rewrite — only a switch from "the parameter is decorative" to "the parameter is checked."

### Slice progress

- **Slice 1 ✅:** `math` module shipped via the built-in prelude as an inline `mod math { pub fn ... }`. Users opt in with `use math::min;` etc. Stdlib functions are written in fastC itself — `abs_i32` / `abs_i64` / `abs_isize` / `abs_f32` / `abs_f64` as non-generic helpers, plus bounded-generic `min[T: Ord]` / `max[T: Ord]` / `clamp[T: Ord]` that work across every numeric primitive via the stage-1.0 `Ord` impls. Required two mono fixes: (a) `MonoCtx::new` now recursively walks `Item::Mod` bodies to discover generic fns nested in modules; (b) pass 2 strips generic-fn declarations from mod bodies before emit so lower doesn't produce literal-`T` C code. `examples/math_demo.fc` compiles and runs (exit 177).
- **Slice 2 ✅:** Doc comments. `///` lines are recognized by the parser and accumulated as `doc_comments: Vec<String>` on every declaration kind (FnDecl, StructDecl, EnumDecl, ConstDecl, TraitDecl, ImplBlock). Trivia lexer filters them out so the formatter doesn't double-print; `fmt` re-emits them canonically. `////` (four+) remains a regular comment per the Rust convention.
- **Slice 3 ✅:** `mem` module — `extern "C"` wrappers for libc `malloc` / `free`, exposed as `mem::alloc(size: usize) -> rawm(u8)` and `mem::free_bytes(ptr: rawm(u8))`. First demonstration that the prelude can carry FFI declarations and that extern blocks parse inside `mod` bodies. `examples/mem_demo.fc` round-trips an allocation and exits 0.
- **Slice 4 ✅:** `io` module — `println(s: raw(u8))` and `put_char(c: i32)` for stdout. Runtime helpers `fc_puts_u8` / `fc_putchar` in `fastc_runtime.h` bridge `raw(u8)` to libc's `char*`. The lower pass now wraps `cstr("...")` literals in an explicit `(const uint8_t*)` cast so `-Werror -Wall` stops complaining. `examples/io_demo.fc` prints `hello, fastC!` and `!\n`. Hello-world without writing FFI by hand finally works.
- **Slice 5 ✅:** Function pointers. `fn(T) -> R` types lower through new `CType::FnPtr`; the emitter pre-pass walks the C AST collecting unique fn-pointer signatures and emits typedefs at the top of the output. `apply(dbl, 5)` and `let f: fn(i32) -> i32 = add_one;` both compile and run end-to-end. `examples/fn_ptr.fc` exercises both higher-order args and local bindings (exit 50). This is the prerequisite for any future closure slice — anonymous `|x| ...` syntax desugars onto these typedef-backed fn ptrs.
- **Slice 6 ✅:** Generic structs (closes the deferred 0.9 item). New `run_struct_mono` post-pass after fn monomorphization. Walks the AST, mangles every `NamedGeneric(Pair, [i32, bool])` to `Named("Pair_i32_bool")`, infers type arguments at struct-literal sites from field values, drops generic struct declarations, and emits one specialized struct per instantiation. `examples/generic_struct.fc` runs two independent instantiations in the same compilation (`Pair[i32, bool]` and `Pair[f64, i32]`, exit 49). Unblocks the path to `vec` / `hashmap` in subsequent slices.
- **Slice 7 ✅:** `vec` — first generic container. `struct Vec[T]` lives in the prelude with a `rawm(T) data; usize len; usize cap` shape; `mod vec` ships `with_capacity` / `push` / `get` / `len` / `release` as bounded-generic free functions. v1 is fixed-capacity (no automatic growth) and uses explicit `vec::release` for cleanup — generic `impl Drop for Vec[T]` is blocked on parser support for impl-on-generic targets, scheduled for a follow-up slice. Required four small but load-bearing additions: (a) a new `sizeof(T)` builtin (lexer/AST/parser/typecheck/lower/mono) returning `usize`, mono-substituted through generic specialization; (b) `Expr::At` typecheck extended to allow `raw(T)`/`rawm(T)` bases (returns `T` inside `unsafe`), so vec can index its raw buffer; (c) `unify_generic` recurses into `NamedGeneric` arguments so `Vec[T]` against `Vec[i32]` binds `T → i32`, and `approx_expr_type` now handles `Addr`/`AddrM`/`Deref` so receiver-typed generic calls like `vec::push(addrm(v), x)` specialize correctly; (d) a new `addrm(x)` builtin returning `mref(T)` — `addr(x)` always produces `ref(T)`, so mutating vec functions like `push` and `release` need a separate way to take a mutable address. Also fixed a lower-pass bug where `use mem::alloc;` inside `mod vec` mangled the call as `vec__mem__alloc` instead of resolving to the root-level `mem__alloc`. `examples/vec_demo.fc` pushes four `i32`s into a capacity-4 vec, reads them back, queries length, releases the buffer, and exits 46.
- **Slice 8 ✅:** Growable vec. `vec::push` now resizes the backing buffer via `mem::resize` (libc `realloc`) when `len == cap`, doubling capacity (initial 4 if cap was 0). Added a `vec::new(seed) -> Vec[T]` constructor that starts empty. Three load-bearing fixes shipped with the growth machinery: (1) mono now looks up callees in `generic_fns` directly instead of going through the symbol table — needed so `vec::new` can call `vec::with_capacity` without an explicit `use` import (the symbol-table check missed mod-internal generic-to-generic calls because they don't live in root scope); (2) the lower pass now registers every mod-nested function under its bare name in `import_map`, so monomorphized generic functions lifted out of the mod (like `push_i32`) still resolve sibling non-generic helpers (`vec::next_cap`); (3) the `if` / `while` emitters reuse the expression's own outer parens when present, avoiding the `if ((x == y))` double-paren pattern that trips clang's `-Wparentheses-equality` warning under `-Werror`. `examples/vec_demo.fc` now pushes six `i32`s starting from an empty vec, forcing two growth events (0→4→8), sums them, and exits 21.
- **Slice 9 ✅:** Vec ergonomics. `vec::pop` returns `opt(T)` (`none` when empty, `some(x)` otherwise — first stdlib API that wraps a raw-pointer read in an option), `vec::clear` resets `len` without freeing, `vec::is_empty` is the cheap zero check, and `vec::contains[T: Eq]` linear-searches via the prelude `Eq` trait. The bounded-generic dispatch reuses the same `T_eq(&cur, target)` machinery that `math::min[T: Ord]` already proved out. Required one lower fix: `infer_expr_type` now handles `Expr::At` / `Deref` / `Addr` / `AddrM`, so `some(at(buf, i))` lowers to a correctly-typed `fc_opt_T` literal instead of degenerating to `fc_opt_void`. `examples/vec_ops_demo.fc` pushes three, pops four (one returning `none`), clears, queries, and exits 76.
- **Slice 10 ✅:** `mod str` — owned byte strings. `struct Str { data: Vec[u8] }` is the first non-generic struct in the prelude that holds a generic struct as a direct field. Mod ships `make` / `with_cap` / `push_byte` / `byte_at` / `byte_count` / `empty` / `dispose` — names diverge from `vec::*` to avoid shadowing because fastC v1 has no `use X as Y`. Four passes had to learn new tricks: (1) mono's pass 1 now recurses into mod bodies to collect generic call sites inside concrete mod-internal functions (`str::push_byte` -> `vec::push`); (2) `strip_generic_fns_from_mod` now rewrites the bodies of surviving non-generic functions through `rewrite_fn` so those call sites get mangled to specialized names; (3) `approx_expr_type`'s `Field` handler now looks up the field's declared type in a new `all_structs` table threaded through `MonoCtx`, so a generic call's receiver `addr(s.data)` can drive T-inference; (4) lower now topologically sorts `type_defs` by direct (non-pointer) field dependencies — the alphabetical sort put `Str` before `Vec_u8`, which broke C's "declared before use" rule. `examples/str_demo.fc` builds "ABC" byte-by-byte and exits 202.
- **Slice 11 ✅:** `vec::map[T, U]` — first higher-order generic. Takes a `ref(Vec[T])` and a `fn(T) -> U` pointer, returns a freshly-allocated `Vec[U]` sized exactly to the input. Type-arg inference drives both legs from a single call: `T` from the receiver, `U` from the mapping function's return type. Two unification improvements: (1) `unify_generic` now recurses into `TypeExpr::Fn` so `fn(T) -> U` against `fn(i32) -> bool` binds both params; (2) `approx_expr_type`'s `Ident` handler now consults a new `all_fns` table on `MonoCtx` to recover the `fn(P) -> R` shape when a bare identifier names a free function. `examples/vec_map_demo.fc` doubles `[1,2,3,4]` to `[2,4,6,8]` and exits 24.
- **Slice 12 ✅:** Vec mutators — `swap` / `reverse` / `filter`. `swap[T](v, i, j)` and `reverse[T](v)` are in-place; both reuse the existing buffer through the same raw-pointer temp-swap pattern. `filter[T](src, pred: fn(T) -> bool)` is the second higher-order on vec, building a fresh `Vec[T]` of just the elements that match — and the first stdlib API to construct an empty vec via direct `alloc(0)` + struct literal so it doesn't need a `seed: T` argument the way `with_capacity` does. `examples/vec_higher_order_demo.fc` swaps, reverses, refills, filters evens, exits 18.
- **Slice 13 ✅:** `vec::sort[T: Ord]` — first bounded-generic mutator on the container surface. Insertion sort dispatches through `cur.less_than(addr(prev))`, which mono lowers to `T_less_than(&cur, &prev)` — the same Ord-dispatch path `math::min` already exercises, but now inside a generic container's in-place mutation. O(n²) is fine for v1 stdlib workloads; a quicksort/introsort replacement waits until generic recursion is exercised. `examples/vec_sort_demo.fc` sorts `[5,2,8,1,9,3]` to `[1,2,3,5,8,9]` and exits 28 only when every slot is in its expected position.
- **Slice 14 ✅:** `vec::for_each[T](v, f: fn(T) -> void)` — side-effect iteration. First stdlib API to take a void-returning fn pointer end-to-end; validates the typedef pre-pass on `fn(i32) -> void` and confirms `unify_generic`'s Fn recursion handles void returns. `examples/vec_for_each_demo.fc` pushes six ASCII codes into a vec and passes `io::put_char` directly as the visitor — prints `FastC\n` to stdout and exits 6 (the element count).
- **Slice 15 ✅:** `vec::reduce[T, U](src, init: U, f: fn(U, T) -> U) -> U` — left fold. First stdlib API to take a two-argument fn pointer; validates the emitter's typedef pre-pass on `fn(i32, i32) -> i32` (typedef name has to encode both param types) and `unify_generic`'s Fn recursion across both parameter positions. `examples/vec_reduce_demo.fc` folds `[1..5]` with `add` (init=0) → 15 and with `mul` (init=1) → 120, exits 135.
- **Slice 16 ✅:** `vec::extend(dst, src)` and `str::eq`. `extend` walks `src` and pushes each element into `dst` — exercises mod-internal generic-to-generic dispatch at one more remove than `vec::new` → `with_capacity` did. `str::eq` reaches through the wrapper to the embedded vec's raw buffer (`(deref(s)).data.data`), validating the new struct-mono field projection on a nested Field chain. `examples/vec_extend_str_eq_demo.fc` concatenates two i32 vecs and compares three Str pairs (equal / different byte / different length), exits 110.
- **Slice 17 ✅:** `Hash` trait + per-primitive impls. `trait Hash { fn hash(self: ref(Self)) -> usize; }` shipped alongside Eq/Ord/Copy/Drop in the prelude, with identity-hash impls for every integer primitive (`u8` through `u64`, `i8` through `i64`, `usize` / `isize`). Signed types cast through their unsigned partner first so `-1` doesn't sign-extend to `usize::MAX` and collide trivially. v1 leaves avalanche mixing to the consumer (`hashmap::hm_bucket_of`); a proper fxhash/wyhash replacement waits until the benchmarking slice surfaces collision-rate numbers.
- **Slice 18 ✅:** `HashMap[K: Hash + Eq, V]` — first stdlib type with two trait bounds on the same type parameter. Open-addressing with linear probing; tombstones keep probe chains intact on remove; rehash doubles capacity when load (occupied + tombstones) exceeds 75%. Public API: `new_map` / `with_cap_map` / `put` / `lookup` / `drop_key` / `has_key` / `count_map` / `empty_map` / `release_map`. The `_map` suffix avoids a collision with `vec::*` because mono's current `generic_fns` lookup keys by bare name; qualified-call resolution is the real fix and is filed for a stage 1.5 cleanup. Compiler shipped one fix alongside: lower's call-site temp generator now uses a pre-built `fn_return_types` map (every mangled-name → C return-type) instead of hardcoding `int32_t`, so `or_zero(drop_key(...))` declares its temp with `fc_opt_int32_t` and clang accepts the assignment. `examples/hashmap_demo.fc` inserts ten entries (forcing a rehash), updates one, removes one, queries an absent key, and exits 129 only when every observation matches its expected value.
- **Slice 19 ✅:** First IO formatting in the stdlib — `io::print_int` and `str::write_line`. `print_int` calls a new `fc_print_i32` runtime helper that walks digits via `putchar` (no `snprintf` so the runtime header stays minimal). `str::write_line` walks a `Str`'s bytes via `put_char` and appends `'\n'` — uses byte-loop output rather than `puts` because `Str` is not null-terminated. `examples/io_format_demo.fc` prints `42`, `-100`, `0`, `fastC` on separate lines.
- **Slice 20 ✅:** `vec::any[T]` and `vec::all[T]` — short-circuiting predicate scans. Both take a `fn(T) -> bool`; `any` returns on first match, `all` returns on first mismatch. Vacuously: `any` on empty is false, `all` on empty is true. `examples/vec_any_all_demo.fc` exits 63 only when all six bool observations match (every-positive, no-huge, vacuous cases, etc.).
- **Slice 21 ✅:** Lower `resolve_ident` is now scope-aware. Local bindings recorded in `var_types` (parameters + every `let`) take precedence over the import map, so a `let empty = ...` no longer gets silently rewritten to `str__empty` when both names exist in scope. Cleanup payoff: the slice-20 demo's `evec` workaround reverts to its natural `empty` binding.
- **Slice 22 ✅:** `vec::min_of[T: Ord]` and `vec::max_of[T: Ord]` (linear scans returning `opt(T)`); `vec::clone[T]` (packed deep copy — `cap = len`, so the clone carries no slack); `str::from_cstr` (build a `Str` by walking an FFI null-terminated `raw(u8)` byte-by-byte). All four are pure stdlib additions; no compiler-pass changes. The `_of` suffix sidesteps the same mono naming collision the hashmap rename did — `math::min` / `math::max` already own the bare names, and qualified-call resolution would let us drop the suffix. `examples/vec_min_max_clone_demo.fc` covers all four and exits 93.
- **Slice 23 ✅:** `str::starts_with` and `str::push_cstr`. `push_cstr` appends every byte of a null-terminated C string to an existing `Str` (the natural in-place sibling of `from_cstr`); `starts_with` is the first stdlib prefix check — useful for parsing, header inspection, prefix stripping. `examples/str_helpers_demo.fc` builds "hello world" from two C-string fragments, exercises four `starts_with` cases (true / mid-word / empty-needle / overlong-needle), exits 111.
- **Slice 24 ✅:** `HashMap[Str, i32]` — first hashmap with a non-primitive key. Two new trait impls in the prelude (`impl Hash for Str` using djb2, `impl Eq for Str` using byte compare) satisfy `HashMap[K: Hash + Eq, V]`'s bounds on a user-defined struct. Mono dispatches every probe step through `Str_hash` / `Str_eq`, validating the trait-dispatch path on a non-primitive key end-to-end. Compiler shipped one fix alongside: the emitter now writes a forward `typedef struct Foo Foo;` for every struct *before* any full definition, so a struct field of type `Other*` works even when topological-sort can't pull the full `Other` definition ahead. `examples/hashmap_str_demo.fc` inserts three string→int pairs, looks each one up via a freshly-allocated key (proving Eq compares bytes not pointers), exits 136.
- **Slice 25 ✅:** `hashmap::for_each_entry[K, V](m, f: fn(K, V) -> void)` (first higher-order on hashmap) and `str::byte_search(s, byte) -> opt(usize)` (first stdlib search returning an optional index). Without closures the demo routes side-effects through a tiny runtime accumulator (`fc_test_acc_add/get/reset`) — a stand-in until captured closures land. `examples/hashmap_iter_demo.fc` sums `(key+value)` across three entries via `for_each_entry`, finds the space in "hello world" via `byte_search`, exits 72.
- **Slice 26 ✅:** Capture-free closures. `|x: T, y: U| -> R { body }` (and the zero-arg `|| -> R { body }`) parse as anonymous fn expressions at primary position. The new `ClosureLifter` walker (running inside `desugar` before resolve) generates a synthetic top-level `Item::Fn` named `__lambda_N` for each closure and rewrites the original expression into `Expr::Ident("__lambda_N")`. From there everything downstream — resolve, typecheck, mono, lower — treats it as a plain top-level fn pointer; no environment struct, no per-call allocation. Captures-by-value arrive in a follow-up slice once the closure environment + dispatch machinery is in. `examples/closure_demo.fc` replaces every helper-fn that earlier map/reduce/filter/for_each demos needed with an inline closure — `map(addr(v), |x: i32| -> i32 { return (x+x); })`, `reduce(addr(v), 0, |acc: i32, x: i32| -> i32 { return (acc+x); })`, etc. — prints `1 2 3 4` to stdout, exits 22.
- **Slice 27 ✅:** `str::split(s, delim) -> Vec[Str]` — first stdlib function returning a generic container *of* a generic struct. Required adding Call-return-type inference to mono's `approx_expr_type`: previously a call like `vec::new(make())` left `T` unresolved because the inner `make()` return type wasn't recovered, so mono defaulted to `void` and emitted broken `new_void(seed: void)`. The new arm consults the `all_fns` table for non-generic callees and substitutes their return type through the active substitution. Plus tiny additions: `vec::sum_i32` / `vec::product_i32` (specialized non-generic helpers — a real bounded-generic `sum[T: Add]` waits on a numeric `Add` trait). `examples/str_split_demo.fc` splits "alpha:beta:gamma" on ':', verifies three segments back with right lengths and first bytes, exits 89.
- **Slice 28 ✅:** String trimming + ASCII case mapping + non-destructive concat. `str::trim_start` / `str::trim_end` / `str::trim` strip ASCII whitespace (space/tab/LF/CR) and return fresh allocations. `str::to_upper` maps every ASCII letter to uppercase, non-letter bytes pass through. `vec::concat[T](a, b) -> Vec[T]` is the non-destructive sibling of `vec::extend(dst, src)` — builds a packed fresh vec from two read-only inputs. `examples/str_trim_upper_demo.fc` (exit 84) and `examples/vec_concat_demo.fc` (exit 18) verify the byte and length outputs.
- **Slice 29 ✅:** `str::repeat(s, count)` builds N copies of a Str. `hashmap::clone_map[K: Hash + Eq, V]` allocates fresh key/value/state buffers and copies every slot bit-by-bit. For primitive K and V the clone is fully independent; for owned types like `Str` the `.data` pointers alias the source (no per-entry deep clone in v1 because there's no `Clone` trait yet — documented in calling code). `examples/str_repeat_hm_clone_demo.fc` verifies independence under primitive keys by mutating the clone and checking the source stays intact, exits 182.

- [x] Closures: `|x: i32| -> i32 { return (x + 1); }`. *Slice 26 — capture-free closures parse at primary position and lift to synthetic `__lambda_N` top-level fns via a new `ClosureLifter` desugar walker, then ride the existing fn-pointer typedef pre-pass. v1 has no captured environment; capture-by-value with stack-allocated env structs is a follow-up slice. `examples/closure_demo.fc` replaces every helper-fn from earlier higher-order demos.*
  - Captures are by value (copy). Mutable captures require `mref` in the closure signature. *(captures: follow-up)*
  - No implicit heap allocation for closures — they are stack-allocated structs.
  - *Slice 5 ✅ (function pointers, the prerequisite):* `fn(T) -> R` is a first-class type. Pass named functions as values, store them in locals, write higher-order helpers like `fn apply(f: fn(i32) -> i32, x: i32) -> i32`. Required a new `CType::FnPtr` plus an emitter pre-pass that walks the full C AST collecting unique fn-pointer types and emits typedefs (`typedef int32_t (*fc_fn_int32_t_to_int32_t)(int32_t);`) at the top of every output.
- [ ] Standard library written in FastC:
  - [x] `io` — `println(s)` + `put_char(c)` for stdout. *Slice 4 — bridges `raw(u8)` to libc's `char*` via static-inline runtime helpers (`fc_puts_u8`, `fc_putchar`). Also added a lowering fix so `cstr("...")` emits an explicit `(const uint8_t*)` cast, clearing `-Wpointer-sign` under `-Werror`. File I/O, stdin, and the capability stub remain for follow-up slices.*
  - [ ] `string` — owned strings, slicing, formatting
  - [x] `vec` — heap-backed array (generic). *Slice 7+8 — `struct Vec[T] { data: rawm(T), len: usize, cap: usize }` in the prelude with `mod vec` exposing `new` / `with_capacity` / `push` / `get` / `len` / `release` as bounded-generic free functions. Slice 8 made `push` growable via `mem::resize` (libc realloc), doubling cap (initial 4) when `len == cap`. `impl Drop for Vec[T]` is still a follow-up — needs parser support for impl-on-generic targets. New `sizeof(T)` builtin, raw-pointer indexing via `at(buf, i)`, `addrm(x)` for mutable addresses, and a fix to `unify_generic` so `Vec[T]` unifies with `Vec[i32]` all came in with these slices. `examples/vec_demo.fc` pushes six i32s starting from empty (two growth events) and exits 21.*
  - [x] `hashmap` — hash table (generic). *Slice 17 + 18 — `Hash` trait shipped with identity impls for every integer primitive, plus `HashMap[K: Hash + Eq, V]` with open-addressing/linear probing/tombstones/grow-by-2x rehashing. Public API uses `_map` / `put` / `lookup` / `drop_key` suffixes to avoid bare-name collisions with `vec::*` in mono's `generic_fns` table — qualified-call resolution is the real fix and is filed for a later cleanup. `examples/hashmap_demo.fc` inserts 10 entries, forces a rehash, updates/removes/queries, exits 129.*
  - [x] `mem` — allocators (`alloc(size)`, `free_bytes(ptr)`). *Slice 3 — wraps libc malloc/free via `extern "C"` inside a `mod mem` block in the prelude. Copy/move helpers deferred until generic structs unblock real container types.*
  - [x] `math` — numeric functions. *Slice 1 — see above.*
  - [ ] `fs` — filesystem operations (capability stub)
  - [ ] `net` — TCP/UDP sockets (capability stub)
- [ ] Iterator protocol via traits + closures.
- [x] Doc comments (`///`) parsed and available to tooling. *Slice 2 — `///` lines accumulate as `doc_comments: Vec<String>` on FnDecl/StructDecl/EnumDecl/ConstDecl/TraitDecl/ImplBlock. Trivia lexer skips them so the formatter doesn't double-emit; fmt prints them back canonically. `////` (four+) remains a regular comment per the Rust convention.*
- [ ] Language specification document.
- [ ] Stability commitment: no breaking changes without a migration path.

**Definition of Done**

- [ ] A non-trivial program (HTTP client or JSON parser) compiles using only the standard library.
- [ ] Standard library has test coverage and documentation.
- [ ] Language specification is published.

## 1.2 — Benchmarking Infrastructure

> **Requires:** 1.1 (real programs to benchmark — toy benchmarks are meaningless).
> **Complexity managed:** Honest performance data — for both runtime *and* the agent workflow. Without benchmarks, "C-like performance" and "agent-friendly" are hand-waving. With benchmarks, we know exactly where safety checks cost performance, and exactly how many tokens a Claude/GPT/Gemini prompt eats to produce a correct program in fastC vs Rust vs Zig vs Go.
> **Complexity refused:** No benchmark-driven optimization. We do not add compiler special-cases to win benchmarks. If bounds checks cost 3% on n-body, we report 3% — and explain why that trade-off is worth it.

Establish a rigorous, reproducible benchmarking framework. See [docs/benchmarking.md](benchmarking.md) for full methodology. **This stage is the launch artifact** — the numbers from 1.2 are what go on Hacker News.

- [ ] `bench/` directory with cross-language benchmark suite.
- [ ] 6 CLBG-style programs: n-body, binary-trees, spectral-norm, mandelbrot, fannkuch-redux, fasta.
- [ ] Micro-benchmarks: array-sum, struct-access, bounds-check overhead, ffi-call.
- [ ] Custom harness: shell/Python orchestrator using `hyperfine` + `perf`.
- [ ] **Token-efficiency benchmark.** Same task in fastC, Rust, Zig, Go: (a) input token count for the equivalent prompt; (b) output token count for a correct program; (c) first-compile success rate for Claude Sonnet 4.6, GPT-5, Gemini 3 Pro on N=50 trials per language.
- [ ] **Agent usability benchmarks** (error recovery rate, code gen accuracy, diagnostic parsability).
- [ ] **Compile-time benchmarks** comparing `fastc+cc` vs `gcc` vs `clang` vs `zig` vs `rustc` on the same HTTP+TLS server program.
- [ ] **Dependency count benchmark.** Total transitive deps and total executable build-script invocations for the same HTTP+TLS server in fastC vs Rust vs Zig vs Go. (Expected: fastC 4, Go 12, Zig 8, Rust 87+.)

**Definition of Done**

- [ ] `./bench/run_all.sh` produces reproducible markdown comparison tables.
- [ ] Benchmarks run in CI with historical tracking.
- [ ] Results are published with hardware specifications, prompt texts, model versions, and methodology notes.
- [ ] One headline number is publishable: "Clean build of an HTTP server with TLS: fastC <X>s, Go <Y>s, Zig <Z>s, Rust <W>s."

## 1.3 — Annotation Mode + Mandatory Module Headers

> **Requires:** 1.1 (stdlib provides the surface to annotate), 1.2 (token-efficiency benchmark validates that annotations are net-positive for agents).
> **Complexity managed:** Every fastC function signature becomes a typed operating manual. The agent never needs to read the body to know what a function does — the signature carries memory region, panic behaviour, purity level, complexity bound, and (later, via 1.4 / 1.5) capabilities and contracts.
> **Complexity refused:** No optional/aspirational annotations. Mandatory on public functions and module headers — the compiler rejects code that omits them. No Java-verbosity tax on private helpers: annotations are inferred and `fastc fmt --annotate` writes the inferred values back into source on demand.

See [docs/annotations.md](annotations.md) for the full grammar specification. This stage lands the **lint-checked** subset (`@mem`, `@panics`, `@purity`, `@complexity` + the module headers). The **proof-checked** subset (`@caps`, `@requires`, `@ensures`) follows in 1.4 and 1.5.

- [ ] First-class annotation grammar (not metadata in comments — parsed as part of the function/module declaration).
- [ ] Function-level annotations: `@mem(arena=...)`, `@panics(never|on=...|always)`, `@purity(pure|effect|io)`, `@complexity(O(...))`.
- [ ] Module-level annotations (mandatory on every module): `@module`, `@owns`, `@arch`, `@depends`, `@threading`, `@invariants`.
- [ ] Module-graph build pass that validates `@owns` globally unique, `@depends` exhaustive, `@arch` layering DAG enforced.
- [ ] `fastc fmt --annotate` infers and writes annotations back into source.
- [ ] `fastc explain <symbol>` emits machine-readable JSON of a function's full annotation surface.
- [ ] All compiler errors for missing/violated annotations carry miette spans + `.with_help()` fix-it hints.

**Definition of Done**

- [ ] A module without a `//! @module` header fails to build with a precise diagnostic.
- [ ] A `pub` function without a complete annotation set fails to build.
- [ ] A private function without annotations builds, and `fastc fmt --annotate` fills them in.
- [ ] `fastc explain` output is sufficient for an agent to call a function correctly without reading its body (verified against the 1.2 token-efficiency benchmark).
- [ ] All annotations in stage 1.1's stdlib are present and pass the new checker.

## 1.4 — Capability System

> **Requires:** 1.3 (annotation grammar landed). Replaces half of the deleted "Effect System" stage.
> **Complexity managed:** Generated code cannot perform arbitrary I/O. Every function's `@caps` set is a typed argument list of capability tokens. Tokens are minted only in `main()` and passed downward. Calling a function that requires a capability you do not hold is a compile error, not a runtime check.
> **Complexity refused:** No algebraic effects (hidden control flow via effect handlers). No monadic effects (Haskell-style, too abstract for a C-like language). No ambient authority — there is no global `fs.read()` you can call without holding a `fs.read` token. The capability lattice has a finite, named set of base capabilities; users do not define new ones in v1.

See [docs/capabilities.md](capabilities.md) for the full design. This is the wedge feature — the property that lets an agent generate fastC code in 2026 with structural confidence that a compromised dep cannot phone home.

- [ ] Capability types built-in: `fs.read(path)`, `fs.write(path)`, `net.connect(host)`, `net.listen(port)`, `proc.spawn`, `time.read`, `rand`, `env.read`.
- [ ] `@caps(...)` annotation parses to a capability set on the function signature.
- [ ] Capability values are first-class types: `cap.fs.read` is a type, instances are tokens.
- [ ] `main()` is the only function that can mint capability tokens (via the runtime `fc_cap_root` interface).
- [ ] Call-graph propagation: callee's `@caps` must be a subset of caller's `@caps`.
- [ ] Token flow analysis: a function declares which of its parameters are capability tokens; the compiler checks that every I/O operation is reached through a token argument.
- [ ] Capabilities erase to zero at runtime (no overhead — they are types, not values, post-codegen).
- [ ] Stdlib (1.1) I/O signatures upgraded from "decorative capability stub" to "checked capability argument."
- [ ] `fastc context` and `fastc explain` include capability sets in their output.
- [ ] `caps.json` artifact emitted per build: the full capability graph of the program.

**Definition of Done**

- [ ] A `@caps()` (pure) function calling `fs_read` produces a compile-time error with a `caps.fs.read` fix-it hint.
- [ ] An HTTP server example compiles where the request handler holds `net.read | net.write` but not `fs.*`, structurally proving it cannot read the filesystem.
- [ ] `caps.json` for a "hello world" program contains exactly the capabilities `main()` minted.
- [ ] No runtime capability check overhead in `--release` mode (verified via 1.2 micro-benchmark).

## 1.5 — Contracts (Runtime Tier)

> **Requires:** 1.3 (annotation grammar landed). Replaces half of the deleted "Effect System" stage.
> **Complexity managed:** Pre- and postconditions on public APIs become first-class. The signature declares not just what a function takes and returns, but what must be true on entry and what is guaranteed on exit. Agents reason from the contract; the compiler enforces it.
> **Complexity refused:** No SMT discharge in v1. That's stage 2.1. v1 lowers every contract obligation to a runtime `assert()` in debug builds and `__builtin_assume` in release. This is the cheap, reliable path — it ships the surface syntax and the diagnostic story without gambling the project on Z3 UX.

See [docs/contracts.md](contracts.md) for the design. The v1 → v2 path is documented up front: every contract written against v1 will be proof-discharged automatically in v2 with no source change.

- [ ] `@requires(<expr>)` and `@ensures(<expr>)` annotations on function signatures.
- [ ] Special `result` keyword in `@ensures` for the return value.
- [ ] Special `old(<expr>)` form in `@ensures` for pre-state references.
- [ ] Contract lowering pass: `@requires` becomes an `assert()` at function entry, `@ensures` becomes an `assert()` at every return.
- [ ] Release mode (`--release`) lowers contracts to `__builtin_assume` (compiler hint, no runtime check) — opt-out via `--check-contracts`.
- [ ] `@invariant(<expr>)` at the module-header level; checked at module boundaries.
- [ ] Per-build `discharge.json` artifact: "discharged via runtime assert: 412 obligations, 0 proven, 0 deferred." (Stage 2.1 will fill in the "proven" column.)
- [ ] Integration with `cert-report`: contract compliance counted as evidence.

**Definition of Done**

- [ ] An `@ensures(result > 0)` function that returns 0 traps with a contract-violation diagnostic in debug builds.
- [ ] Contract violations produce the same structured diagnostic quality as type errors (miette spans, fix-it hints).
- [ ] `discharge.json` is consumed by the MCP server (stage 1.6).
- [ ] Stdlib functions have complete `@requires` / `@ensures` coverage.

## 1.6 — Agent-First Features + MCP Server

> **Requires:** 1.1 (real language to work with), 1.3 (annotation surface), 1.4 (capability graph), 1.5 (contract discharge report). All three artifacts (`manifest.json`, `caps.json`, `discharge.json`) become MCP resources here.
> **Complexity managed:** The gap between "compiler says there's an error" and "the error is fixed," extended to "the agent has full structural context without re-deriving it." Today, an agent runs `cargo check` and parses text. With `fastc-mcp`, the agent queries the AST, capability graph, contract discharge, and fix suggestions over a typed protocol.
> **Complexity refused:** No AI inside the compiler. `fastc fix` applies deterministic fix-it hints, not LLM suggestions. The compiler remains a pure function from source to output. Agent intelligence lives in the agent, served fastC context by `fastc-mcp`.

Make FastC the best language for AI coding agents. See [docs/agent-features.md](agent-features.md) and [docs/mcp.md](mcp.md) for full specifications.

- [ ] Extend `--output-format=json` from `cert-report` to all CLI commands (`compile`, `check`, `fmt`, `explain`).
- [ ] `fastc fix` command — auto-apply the existing `.with_help()` fix-it hints from diagnostics.
- [ ] `fastc context` — dump project type surface for AI context windows.
- [ ] `fastc diff` — semantic code diff (AST-level, not text-level).
- [ ] `fastc explain <symbol>` — full annotation surface as JSON.
- [ ] Inline `test { }` blocks compiled only in test mode.
- [ ] LSP enhancements: code actions (from fix-it hints), semantic tokens, workspace rename.
- [ ] Unify `CompileError` diagnostics, `P10Violation` reports, capability errors, and contract violations into a single JSON diagnostic stream.
- [ ] **`fastc-mcp` server** (new `crates/fastc-mcp/`) exposing AST, types, capabilities, contracts, and fix suggestions as MCP resources. Reads `manifest.json` / `caps.json` / `discharge.json` from the build cache.
- [ ] Scaffold an `AGENTS.md` file by default from `fastc new` with project conventions.

**Definition of Done**

- [ ] An agent can iterate `check → fix → check` to reach working code without human intervention.
- [ ] `fastc-mcp` is callable from Claude Code, Cursor, and any other MCP-speaking client.
- [ ] All CLI output is machine-parseable when `--output-format=json` is passed.
- [ ] JSON diagnostic format covers compiler errors, safety violations, P10 compliance, capability violations, and contract violations in one stream.

## 1.7 — Vendor-First Package System with Sigstore + SLSA L3

> **Requires:** 1.1 (stable language — packages need a stable API surface), 1.4 (capabilities — the `fastc add` flow displays caps before install), 1.6 (`fastc-mcp` — package metadata flows through the same channel).
> **Complexity managed:** Code reuse without the supply-chain attack surface that has dominated Rust, npm, and PyPI in 2025/2026. Dependencies are git URL + commit hash + content hash, vendored into the user's repo. No central registry to phish, no account to compromise, no typosquatting (the URL is part of the import).
> **Complexity refused:** No HTTP package registry (initially). No semver SAT solver. No build scripts during install. No binary distribution. No platform-specific package variants. The package manager is a glorified `git clone` with content-hash verification.

See [docs/supply-chain.md](supply-chain.md) for the full story.

- [ ] `fastc.toml` dependency entries: `name = { git = "<url>", rev = "<commit>", sha256 = "<hash>" }`.
- [ ] `fastc fetch` — clone deps into `vendor/`, verify content hashes.
- [ ] `fastc add <github-url>` — capability-aware add flow. Before fetching, parses the dep's `fastc.toml`, computes its capability closure, and prompts: "this package requires `fs.read("~/.config/")`, `net.connect("api.example.com")`. Approve?"
- [ ] Build-system constraint: dependency builds use the same `fastc` pipeline. No `build.rs`-equivalent. No proc macros. No postinstall.
- [ ] Reproducible-build verification: hash the C output of a dep build; same source + same `fastc` version produces identical bytes.
- [ ] Global build cache keyed by `(fastc_version, dep_content_hash, target_triple)`.
- [ ] Sigstore signing on `fastc` compiler binary releases.
- [ ] SLSA Level 3 provenance for the compiler binary and stdlib build artifacts.

**Definition of Done**

- [ ] `fastc add github.com/Skelf-Research/fastc-http` works end-to-end: fetches, displays capabilities, verifies hash, vendors, compiles.
- [ ] A user replays a clean build of any fastC project on a fresh machine and gets a build-cache hit, not a rebuild.
- [ ] The compiler binary has verifiable SLSA L3 provenance on the GitHub release page.
- [ ] A canary "malicious package" test confirms that hash mismatch fails the build before any code is compiled.

## 1.8 — fastc-core Curated Stdlib Extensions

> **Requires:** 1.7 (vendor-first package system live so the curated packages have somewhere to live).
> **Complexity managed:** Users get one canonical, audited answer for HTTP, JSON, TOML, logging, CLI parsing, crypto primitives, regex, async runtime, and common data structures. No "Axum vs. Actix vs. Rocket" agent confusion. Every `fastc-core` package is reviewed, signed, capability-typed, and contract-annotated.
> **Complexity refused:** No community-blessing for the first two years. The answer to "is there a fastC library for X" is "yes, in fastc-core" or "no, write it locally." We resist the urge to bless community packages until they have been around for a year and audited.

See [docs/ecosystem.md](ecosystem.md) for the full curation strategy and target package list.

- [ ] **Launch set (week 3–4 of the 8-week plan):** `fastc-http`, `fastc-json`, `fastc-toml`, `fastc-log`, `fastc-cli`.
- [ ] Each package: complete annotation coverage, capability-typed I/O, contract-annotated public functions, Sigstore-signed releases, `AGENTS.md` documenting the canonical idiom.
- [ ] **Six-month set:** add `fastc-sqlite`, `fastc-crypto-primitives`, `fastc-regex`, `fastc-uuid`, `fastc-time`, `fastc-base64`.
- [ ] **One-year set:** add async runtime, TLS, websocket, csv, gzip, ed25519, x509 parser, and the remaining ~15–25 packages to reach the 30–50 target.
- [ ] `fastc.dev` search frontend over GitHub repos matching the `fastc-<name>` convention. No registry to operate.

**Definition of Done**

- [ ] The 5 launch packages exist on GitHub under `Skelf-Research/fastc-core`, signed, with `AGENTS.md` and full annotation coverage.
- [ ] A new fastC project can implement an HTTP+JSON CRUD service using only `fastc-core` packages.
- [ ] `fastc.dev` returns relevant results for "http", "json", "logging" within 1 second.

## 2.0 — Compiler Hardening + Incremental

> **Requires:** 1.7 (ecosystem feedback reveals real-world compiler bugs and pain points).
> **Complexity managed:** Trust. Users cannot adopt fastC for serious work until the compiler itself is proven reliable. This stage makes the compiler trustworthy, not the language more powerful.
> **Complexity refused:** No new language features in this stage. All effort goes into proving what already exists works correctly.

- [ ] Compiler fuzzing with `cargo-fuzz` to find crash bugs and miscompilations.
- [ ] Dedicated fuzz target for the annotation parser (1.3) and capability checker (1.4).
- [ ] Debug info / source maps (C line → fastC source) for debugger integration.
- [ ] Reproducible-build verification on the compiler itself (build the compiler with itself + gcc, hash the output, match across machines).
- [ ] Cross-compilation support (target triples, sysroot configuration).
- [ ] Incremental compilation hardening — extend the 0.8 Salsa skeleton to handle multi-package workspaces with cross-package change propagation.

**Definition of Done**

- [ ] Compiler passes 72-hour fuzzing run with no crashes or miscompilations.
- [ ] Incremental compilation provides measurable speedup (>2×) on projects with 10+ modules.
- [ ] `gdb` / `lldb` can step through fastC source using generated debug info.
- [ ] A canary "rebuild the compiler from itself on three machines" test produces bit-identical binaries.

## 2.1 — SMT Contract Discharge

> **Requires:** 1.5 (contracts as runtime asserts), 2.0 (compiler hardened — SMT is a new failure surface that needs the rest of the compiler stable).
> **Complexity managed:** Contracts get *proved*, not just runtime-checked. A function with `@requires(x > 0)` calling a callee with `@requires(y >= 1)` is discharged at compile time when the call site has `if x > 0 { f(x) }`. The build emits a per-function report: proven N, runtime-checked M, deferred K.
> **Complexity refused:** No mandatory SMT. The `--no-prove` flag skips Z3 entirely and falls back to runtime asserts (the 1.5 behaviour). This is critical for the agent inner loop: agents iterate fast, they want SMT on CI, not on every save.

See [docs/contracts.md](contracts.md) for the three-tier discharge design.

- [ ] Z3 (or comparable SMT solver) wired into a new `contract_discharge` compiler pass.
- [ ] Three-tier pipeline per obligation: syntactic pattern-matching first, then SMT with a 500ms-per-obligation budget, then runtime fallback.
- [ ] Discharge results cached in `.fastc/cache/` keyed by formula hash. Re-running the build does not re-prove.
- [ ] `discharge.json` per build report populated with `proven` and `deferred` columns (1.5 only populated `runtime-checked`).
- [ ] `--no-prove` flag: skip SMT entirely, fall back to 1.5 runtime behaviour. Default in `fastc check` for fast inner-loop development.
- [ ] `--prove-budget=<ms>` flag: override the 500ms per-obligation budget.
- [ ] Readable diagnostics: when SMT times out or returns `unknown`, the error message identifies the obligation and offers a fix-it hint ("strengthen `@requires` to include..." or "weaken `@ensures`...").

**Definition of Done**

- [ ] `discharge.json` for a typical 5000-line fastC program shows >80% of obligations proven syntactically or via SMT, with the rest documented as runtime-checked.
- [ ] CI runs full SMT discharge; developer inner loop uses `--no-prove`.
- [ ] An obligation that times out produces a structured diagnostic with a concrete hint, not a stack trace.

## 2.2 — Safety-Critical Certification

> **Requires:** 2.0 (compiler hardening — certification bodies require evidence of compiler reliability), 2.1 (SMT discharge — auditors get proven contracts, not just runtime asserts).
> **Complexity managed:** Regulatory compliance. fastC's transpilation model is a genuine advantage here: certify the C output with an already-qualified C compiler, rather than qualifying an entire new compiler backend. Contracts + capabilities make the certification story materially stronger than the C-only baseline.
> **Complexity refused:** fastC does not become a "certification framework." It produces evidence (traceability reports, P10 compliance data, contract discharge reports, capability graphs, test coverage metrics) that feeds into existing DO-178C / IEC 62304 / ISO 26262 processes. The certification workflow is the user's responsibility — fastC provides the artifacts.

- [ ] DO-178C / IEC 62304 certification evidence package.
- [ ] Traceability: fastC source line → C output line → binary instruction.
- [ ] P10 compliance reports integrated into certification artifacts.
- [ ] Contract discharge reports (`discharge.json`) integrated as verification evidence.
- [ ] Capability graphs (`caps.json`) integrated as I/O isolation evidence.
- [ ] Formal verification integration (CBMC / Frama-C on emitted C11).

**Definition of Done**

- [ ] A reference project (e.g., flight controller or medical device driver) passes certification review using fastC-generated evidence.
- [ ] Formal verification can prove absence of runtime errors on a 500-line fastC program.
- [ ] An auditor can verify, from `caps.json` alone, that a "no network" subsystem never reaches `net.*` capabilities.

## 2.3 — Async/Await (Optional, Explicit)

> **Requires:** 1.1 (closures for callbacks, traits for a `Future` trait, `Drop` for cancellation cleanup). Benefits from 1.4 (`async fn` is `caps(time.read | net.read | net.write | ...)` — capability typing makes the I/O surface of an async function visible in its signature).
> **This is the hardest feature on the roadmap.** It directly tensions with FastC's core principles:
>
> - **"Explicit effects"** — async introduces hidden suspension points. Every `await` is an invisible `return` + resume.
> - **"No hidden control flow"** — an `async fn` looks like a normal function but executes as a state machine.
> - **"Local reasoning"** — you cannot understand an async function without understanding the executor that runs it.
> - **"Toolchain continuity"** — the C output for async is a state machine struct, not readable sequential code.
>
> **How we reconcile this:** FastC does not hide the complexity. The approach is *explicit coroutines*, not invisible async transformation.

**Design constraints (non-negotiable):**

1. **No colored functions.** An async function is not a different kind of function. It returns a `Future[T]` — a struct that can be polled. The caller decides whether to poll it synchronously or schedule it on an executor. There is no split world of "async functions" vs. "sync functions."

2. **No implicit executor.** There is no built-in runtime. `Future.poll()` is a trait method. Users provide their own event loop, or use a library. FastC ships a minimal single-threaded executor as an *example*, not as standard library.

3. **Visible state machine.** The C output for an async function is an explicit `struct` with an enum state tag and a `poll` function. A developer (or agent) can read the generated C and understand the control flow.

4. **Cancellation via Drop.** Dropping a `Future` cancels it. Cleanup runs the same `Drop` path as any other owned resource. No special cancellation API.

5. **No hidden allocation.** Futures are stack-allocated by default. Boxing a future for dynamic dispatch is explicit: `own(Future[i32])`.

**Implementation approach:**

- [ ] `Future[T]` trait: `fn poll(self: mref(Self)) -> res(T, Pending);`
- [ ] `async fn` syntax sugar that lowers to a state machine struct implementing `Future[T]`.
- [ ] `await` keyword that lowers to a `poll()` call + state transition.
- [ ] Minimal example executor in stdlib examples (not in stdlib itself).
- [ ] Cancellation semantics: drop the future → drop captured state via `Drop` trait.

**Definition of Done**

- [ ] An async TCP echo server compiles and runs using a user-provided event loop.
- [ ] The generated C for an async function is a readable state machine (struct + poll function).
- [ ] An agent can generate working async code using `fastc context` output (the `Future` trait surface is sufficient context).
- [ ] No executor is required to use `Future` — synchronous `poll()` works.

## 2.4+ — Long-Term

Features that depend on ecosystem maturity and community feedback.

- [ ] WASM target via Emscripten or direct C-to-WASM pipeline.
- [ ] `comptime`-style constant evaluation beyond current `const` expressions (only if it can be kept explicit).

These are deliberately vague. They will be specified when the prerequisites exist and community demand is clear.

---

## Competitive Context

See [docs/competitive-analysis.md](competitive-analysis.md) for detailed positioning against C, Zig, Rust, and V, and [docs/MANIFESTO.md](MANIFESTO.md) for the launch thesis.

fastC's core differentiator is the **fusion of capability-typed I/O, mandatory contracts, zero-executable-build-scripts, and capability-aware dependency management** — measured against a strict compile-time budget and served to AI agents over a native MCP protocol. No other systems language combines these properties. Rust has cargo and the borrow checker but pays a permanent tax in compile time, `build.rs`, proc macros, and a 150K-crate supply-chain surface. Zig is small but has no provenance story and runs arbitrary code in `build.zig`. C has 50 years of ecosystem and no safety. fastC occupies the open quadrant: small surface, safe by construction, provable, and built for the age of agent-generated code.
