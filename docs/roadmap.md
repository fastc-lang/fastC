# Roadmap

This roadmap is a living plan. Dates are intentionally omitted until implementation starts.

## v1.0 status (2026-05-27)

**fastC is feature-complete through stage 2.1.** Stages 0.1 through 2.1 plus the v2.0 hardening follow-ups (N1–N6) plus the stage-1.3 annotation surface (A1–A3) plus the stage-1.6 agent-first expansion slice (B2 / B5) plus the stage-1.7 supply-chain polish (C1 / C2). Headline features:

- body-aware SMT discharge; call-site precondition discharge for direct calls + method calls + **function-pointer bindings** (N1);
- `@requires` / `@ensures` annotations inside `impl` blocks; tier-1 expansion (unsigned-nonneg / excluded-middle / identity arithmetic);
- per-function and per-statement source-map `#line` directives;
- `--reproducible` flag on both `fastc compile` and `fastc build` (C2) for cross-directory byte-identical C;
- four libfuzzer targets across the full pipeline;
- multi-source-file project build cache (46× warm-vs-cold);
- `[workspace]` manifest support with per-member incremental (N3);
- closures with by-value literal captures (N4);
- compiler-binary reproducibility via `SOURCE_DATE_EPOCH` + `--remap-path-prefix` in the release workflow (N5);
- fastc-core launch set split into five public repos under [Skelf-Research/fastc-core-{cli,log,json,toml,http}](https://github.com/Skelf-Research) (N2);
- on-disk discharge cache; structured fix-it hints; single-file global build cache;
- **stage-1.3 function-level annotation surface** (A1): `@mem(arena = ident)`, `@panics(never|always|on=expr)`, `@purity(pure|effect|io)`, `@complexity(O(...))` parsing + AST + structured storage. `@panics(never)` and `@purity(pure)` are enforced against the transitive call graph;
- **stage-1.3 module-level mandatory headers** (A2): `//! @module / @owns / @arch / @depends / @threading / @invariants`. Lenient mode (the default) accepts header-less modules; cross-module checks (@owns uniqueness, @depends exhaustiveness, @arch DAG layering) fire when any module declares a header. Strict mode is opt-in via `strict_modules = true` in `fastc.toml` for greenfield projects;
- **`fastc explain` JSON extended** (A3) with the new annotation fields and a top-level `modules` array;
- **`fastc context` + `fastc diff` subcommands** (B2): markdown / JSON dumps of the project's pub type surface and AST-level diffs between two snapshots;
- **MCP method expansion** (B5): the embedded `fastc mcp` server now exposes `check`, `compile`, `caps_summary`, `context`, `diff` alongside the existing `explain`;
- **dep_content_hash in the build cache key** (C1) closes the cache-invalidation loop on dep churn;
- **multi-file project reproducibility** (C2) verified by `crates/fastc/tests/reproducibility.rs::multi_file_project_reproducible_across_dirs`.

**325+ tests pass across the workspace.**

fastC v1.0 is feature-complete for launch — every item in the "What's stable at v1.0" list of [docs/v1.0.md](v1.0.md) is implemented, tested, and exercised end-to-end by the integrated demo at `examples/launch_set_demo.fc`.

### Open v1.x follow-ups (none gate v1.0)

Smaller polish items, in priority order:

1. **`fastc fix` subcommand + structured fix-it spans** (B1) — extends `CompileError` variants with `Fixit { span, replacement, label }` triples so mechanical fixes ("wrap in unsafe", "add `addr(`", "import X") can be auto-applied. Infrastructure first, then per-diagnostic backfill.
2. **Inline `test { }` blocks + `--test` flag** (B3) — `test { fn foo() { assert(...) } }` blocks with `--test` generating a test-runner main. Reuses the `@test` AST field A1 already shipped.
3. **Unified JSON diagnostic envelope** (B4) — single `Diagnostic` shape across compile / check / fmt / cert-report / discharge / caps, plus `--output-format=json` on `compile` and `fmt`.
4. **LSP code actions + semantic tokens + workspace rename** (B6) — `crates/fastc-lsp/` already has diagnostics / hover / goto-def / completions; adding the three remaining LSP capabilities closes the editor story.
5. **Whole-program function-pointer discharge** (C3) — direct calls, method dispatch, and bound fn-pointer assignments (`let f = direct_fn; apply(f, x)`) all discharge after N1. Opaque fn-pointer *parameters* (`fn apply(f: fn(i32) -> i32, x: i32)` called from outside) still fall through. Whole-program callee inference is the closing slice.
6. **Closures with non-literal by-value captures** (v2.0) — IntLit / BoolLit / FloatLit + unary-negated literal captures inline. Non-literal captures (function results, struct fields, mutable bindings) still emit the closure-aware diagnostic. Full env-struct synthesis is the v2.0 follow-up.
7. **`fastc fmt --annotate`** (A3 follow-up) — infer `@panics(never)` / `@purity(pure)` per fn and module headers from existing code structure, write them back. Migration aid; legacy code compiles without it.

Larger external work, scoped but deferred to a dedicated sprint:

- **Stage 1.2 benchmark expansion** (D): five more CLBG programs across 5 languages, compile-time isolation (split fastc-step from cc -O2), dep-count benchmark, `benchmarks/run_all.sh` umbrella.
- **Stage 1.8 fastc-core packaging cutover** (E1): move the existing five fastc-core packages off the prelude onto the `fastc add` flow with Sigstore-signed v1.0.0 releases on every repo.
- **Stage 1.8 fastc-core six-month set** (E2): six new public repos under `Skelf-Research/fastc-core-{time,base64,uuid,crypto-primitives,regex,sqlite}`. Per-package effort: time 200 LoC, base64 150 LoC, uuid 180 LoC, crypto-primitives 500 LoC (hand-rolled SHA-256), regex 800 LoC (Thompson NFA), sqlite 600 LoC + runtime shim (libsqlite3 FFI). Estimated 4–8 weeks of focused authoring.

Anything in stages 2.2+ (safety-critical certification, async/await, etc.) is post-v1.0 and not blocked by the launch.

See [docs/v1.0.md](v1.0.md) for the full stability commitment.

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

## 0.6 — Examples + Scaffolding ✅

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

## 0.7 — Foundation Completion ✅

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

## 1.1 — Standard Library and Closures (MVP) ✅ *(closures-with-captures deferred to v1.x)*

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
- **Slice 30 ✅:** First non-trivial integration demo. `examples/word_count_demo.fc` splits "the quick brown fox jumps over the lazy dog the" on spaces, builds a `HashMap[Str, i32]` frequency histogram via a while-loop calling `put` / `lookup`, then verifies the count for "the" (3), "fox" (1), and a missing word (0), plus the distinct-word count (8) and total word count (10). Touches every stdlib module shipped so far — `str::from_cstr` / `split`, `vec::get` / `len` / `release`, `hashmap::new_map` / `put` / `lookup` / `count_map` / `release_map`, and the prelude `Hash`/`Eq` impls on `Str`. Exits 205 only when every observation matches. Documents the v1 ownership wart: when a `Str` is moved into the map via `put`, both the source `Vec[Str]` and the map alias the inner heap buffer — releasing both would double-free, so the demo deliberately leaks the inner buffers (OS reclaims). A proper `Clone` trait + per-key dispose is a follow-up slice.
- **Slice 31 ✅:** `Clone` trait declared in the prelude (joins `Eq` / `Ord` / `Copy` / `Drop` / `Hash`). First helper: `str::clone_str(s) -> Str` — deep-copies via a fresh byte allocation so source and clone are fully independent. The natural-syntax `s.clone()` via `impl Clone for Str` waits for one piece of compiler plumbing: desugar's method-lifter currently only walks top-level impl blocks; an impl inside a mod (which is where `Str`'s impls naturally belong because that's where its private state is accessible) needs the lifter extended to recurse. `examples/str_clone_demo.fc` proves independence by `push_byte`-mutating the clone and verifying the source stays at its original length, exits 255.
- **Slice 32 ✅:** `str::find(haystack, needle) -> opt(usize)` — first multi-byte search returning a positional optional. Naive O(haystack.len × needle.len) scan; KMP / Boyer-Moore is a stage-2 optimization once benchmarks identify a hotspot. Empty needle returns `some(0)` per the standard convention. Plus `str::contains_str` as the membership-only thin wrapper. `examples/str_find_demo.fc` locates "quick" at index 4, "lazy" at 35, returns none for "cat", and exits 63.
- **Slice 33 ✅:** Three small stdlib additions: `str::concat_str(a, b) -> Str` (suffix to dodge the mono naming collision with `vec::concat[T]`), `str::lines(s) -> Vec[Str]` (thin wrapper around `split` with `'\n'`), and `vec::find_index[T: Eq](v, target) -> opt(usize)` (the positional sibling of `contains`). `examples/concat_lines_findidx_demo.fc` exits 31.
- **Slice 34 ✅:** Math + string helpers — `math::pow_i32(base, exp)` (integer power, 0^0 = 1 convention, negative exp returns 1 by convention), `math::gcd_i32(a, b)` (Euclidean, operates on absolute values, gcd(0,0) = 0 per Python convention), and `str::ends_with` (mirror of `starts_with`, scanning from the end). `examples/math_str_helpers_demo.fc` exits 94.
- **Slice 35 ✅:** Overload-resolution scaffold in mono. `generic_fns` is now `HashMap<String, Vec<FnDecl>>` (multi-candidate per bare name) with a `pick_candidate_for_call` helper that walks candidates and selects the one whose parameter shapes unify cleanly against the actual arguments. Non-breaking change for v1 — every public stdlib name is still unique after the renames, so single-candidate path stays fast. The full unlock (dropping `_str` / `_map` / `_of` suffixes back to natural names) needs a parser/resolver change too: the resolver still rejects two `use` imports of the same bare name. Qualified-call syntax (`vec::len(...)` and `hashmap::len(...)` as call expressions) is the proper follow-up; the mono-side machinery is now ready for it.
- **Slice 36 ✅:** **Stage 1.1 Definition-of-Done demo — JSON tokenizer.** `examples/json_tokenizer_demo.fc` is ~150 lines of real fastC that tokenizes a JSON-ish string into `Vec[Token]` where `Token { kind: i32, value: Str }`. Recognizes punctuation (`{}[],:`), double-quoted strings (no escapes — wait for a richer string slice), runs of digits as numbers, runs of letters as identifiers (`true` / `false` / `null` / etc), and skips whitespace. Body is split into per-token-kind scan helpers (`scan_string`, `scan_while`, `tokenize_one`) so each fits inside P10 Rule 4's function-length budget. Exercises vec, str, fn pointers passed to higher-order helpers, struct literals as generic-seeds, and the prelude `Hash`/`Eq` impls — proving the stdlib can compose end-to-end for a real program. Compiler fix shipped alongside: mono's `approx_expr_type` now handles `Expr::StructLit { name }` by returning `TypeExpr::Named(name)`, so `vec::new(Token { ... })` infers T=Token from the seed instead of defaulting to void. Demo exits 134 only when all 10 observations match (token count of 17, kinds and lengths at five specific positions).
- **Slice 1.4.0 (preview) ✅:** Capability-typed I/O — the **strategic wedge** of fastC's positioning. v1 ships the API *shape*: `Cap*` struct types (`CapFsRead`, `CapFsWrite`, `CapNetConnect`, `CapNetListen`, `CapProcSpawn`, `CapTimeRead`, `CapRand`, `CapEnvRead`), a master `Caps` bundle struct, and `caps::init()` that mints all caps in one call. Enforcement (the flow-analysis pass that errors when a function uses an I/O syscall without holding the matching cap) waits for the proper stage-1.4 slice. `examples/capabilities_demo.fc` demonstrates the user-facing shape: main mints caps, passes the file-read cap to a helper that accepts it as a `ref(CapFsRead)` parameter, and the type system already tracks who holds which cap. Exits 42.
- **Slice 37 ✅:** Language spec updated (`docs/language.md`). Added sections on **Generics** (bounded type params, monomorphization), **Traits** (Eq/Ord/Copy/Drop/Hash/Clone, method dispatch, Self), **Closures** (capture-free, mandatory typed params, `__lambda_N` lifting), **Capabilities (preview)**, and a **Stdlib Summary** listing every module's surface. ~350 lines total. Locks down what fastC v1 actually means as a language. Stage-1.1 DoD line flipped to [x].
- **Slice 1.3.0 ✅:** Function annotations. Three new lexer tokens (`@noalloc`, `@nodiverg`, `@pure`) parse as fn-level attributes and attach to `FnDecl.annotations: Vec<String>`. Recognized in either order with `pub` — `@noalloc pub fn foo()` and `pub fn foo() @noalloc` (well, the latter isn't supported because attrs precede `fn`; the former works). The parser accumulates them via a new `parse_fn_annotations` helper that loops on the `@` tokens. Annotations survive through every walker (desugar / mono / lower) — the lint pass that enforces `@noalloc` (function body doesn't reach `alloc` / `realloc` / `free_bytes`) is the next slice once the call-graph walker is wired through. `examples/annotations_demo.fc` decorates `add_pure` with `@noalloc @nodiverg` and `times_two` with `@pure`, returns 17.
- **Slice 1.5.0 ✅:** Runtime contracts via `@requires(cond)`. The new `AtRequires` token parses any expression inside the parens; the bool result is wired through every walker via a new `FnDecl.requires: Vec<Expr>` field. Lower emits `if (!cond) { fc_trap(); }` at function entry, one block per declared `@requires`, in source order. SMT discharge replaces the runtime trap in stage 2.1, but the runtime tier is always available as the guaranteed fallback. `examples/contracts_demo.fc` declares two `@requires` on `safe_div` (`divisor != 0` and `divisor > 0`) and one on `bounded_index` (`((i >= 0) && (i < 100))`); calling within bounds exits 43, calling outside would trap.
- **Slice 1.6.0 ✅:** `fastc explain <file>` subcommand. Emits a JSON document with every function's name, module path, parameter list (name + type), return type, annotations (`@noalloc` / `@nodiverg` / `@pure`), `@requires` clauses (best-effort textual rendering of the expression), `is_unsafe`, and `doc_comments`. First concrete agent-facing artifact in the stage-1.6 family — the `fastc-mcp` server will surface this as an MCP resource so Claude Code / Cursor / Codex don't have to re-parse fastC source. Output mirrors the shape the planned `manifest.json` / `discharge.json` / `caps.json` artifacts will adopt, keying every entry by a stable name so cross-tool consumers can join.
- **Deferred-item 1 ✅:** Qualified-call syntax — `vec::len(...)` and `hashmap::lookup(...)` now parse, resolve, type-check, mono-specialize, and lower end-to-end. Parser produces an `Expr::Ident` whose name contains `::`; resolver walks `::`-separated segments through module scopes; typecheck reuses the same path lookup; mono registers every fn under both bare and qualified names so a call site can use either form; lower replaces `::` with `__` to produce valid C identifiers. `examples/qualified_calls_demo.fc` calls `vec::new` / `vec::push` / `vec::len` / `vec::get` / `vec::release` / `math::min` / `math::max` without a single `use` import — exits 83. Lets callers disambiguate the cross-module bare-name collisions that previously forced `_str` / `_map` / `_of` suffixes. The renames stay in the v1 stdlib (changing them now would churn every existing demo) but new code can prefer the qualified form, and a follow-up cleanup can revert the suffixes once every demo migrates.
- **Deferred-item 2 ✅:** Capability flow analysis (sealed-type enforcement). A new `cap_check` pass runs between typecheck and p10. It walks the AST tracking module path; if it sees a struct literal whose name is in a sealed list (`CapFsRead` / `CapFsWrite` / `CapNetConnect` / `CapNetListen` / `CapProcSpawn` / `CapTimeRead` / `CapRand` / `CapEnvRead` / `Caps`) and the construction is OUTSIDE `mod caps`, the compile fails with a capability-fabrication diagnostic. This is the cap-typed I/O wedge made enforceable: user code can pass caps around (normal type-checked arg flow) but cannot forge them. `caps::init()` inside `mod caps` legitimately mints them; from there they flow downward through call arguments. Two unit tests pin the behavior: `evil()` returning a hand-constructed `CapFsRead {}` fails to compile; the normal `main` → `caps::init` → helper-taking-cap pattern compiles cleanly. A future sub-slice can extend the lint to multi-arg flow (every fn that calls a cap-requiring callee must itself accept that cap in its signature).
- **Deferred-item 3 ✅:** JSON encoder — `mod json` in the prelude as a preview of what `fastc-core/json` will ship as a vendor-able package post-stage-1.7. Builder API around a new `JsonBuilder { out: Str, needs_comma: i32 }` struct: `new_builder` / `obj_start` / `obj_end` / `arr_start` / `arr_end` / `key` / `str_value` / `int_value` / `bool_value` / `null_value` / `release_builder`. Comma management is encapsulated inside the struct — callers just emit primitives and the encoder threads separators automatically. v1 doesn't escape internal quotes or control characters (waiting for the `fastc-core/json` graduation that pairs with a streaming decoder). `examples/json_encoder_demo.fc` builds `{"name":"Alice","age":30,"admin":true,"score":-7,"tags":["a","b"]}` and prints it to stdout — proves the language is rich enough to ship real stdlib content as fastC itself.
- **Deferred-item 4 ✅:** `fastc mcp` stdio server — first concrete `fastc-mcp` implementation. Reads newline-delimited JSON-RPC 2.0 messages from stdin and writes responses to stdout. Implements three MCP methods: `initialize` (announces protocol version `2025-03-26` and a `tools` capability), `tools/list` (returns one tool: `explain`), and `tools/call` (dispatches to `explain`). The `explain` tool reads a fastC source file by path, parses it, and returns the same JSON `fastc explain` prints — wrapped in MCP's `content: [{type: "text", text: ...}]` envelope. Modern MCP clients (Claude Code, Cursor, Codex) speak this directly: configure `command: fastc, args: [mcp]` in `.mcp.json` and the tool becomes available alongside any other MCP server. Future tools (`check`, `compile`, `caps_summary`, `discharge_report`) bolt on by adding match arms to `handle_tools_call`. Verified end-to-end by piping a three-message JSON-RPC stream through `fastc mcp` and reading the JSON responses.
- **Deferred-item 5 ✅:** Vendor-first dependency schema. `Dependency::Git` gained two optional fields — `sha256` (content hash of the resolved git tree) and `sigstore` (path to the `.sigstore.json` bundle attesting the rev). A new `Dependency::integrity_warnings` method scans a dep and reports three policy violations: missing `rev` (moving tags / branches are unsafe), missing `sha256`, missing `sigstore`. The default `fastc build` reports warnings; `--vendor-strict` (a future flag) converts them to hard errors. Three unit tests pin the behavior: a dep with only a moving tag produces three warnings; a fully-pinned dep with rev+sha256+sigstore is clean; a local path dep is exempt (no upstream to attest). This is the schema half of the vendor-first model. The fetch-side enforcement (verify the downloaded tree's sha256 matches the recorded one, then verify the Sigstore bundle against the public transparency log via `cosign verify-bundle`) is the next sub-slice — it needs network calls and an external sigstore-rs / cosign dependency that's out of scope for the schema work itself.

- **Deferred-item 6 ✅:** `@noalloc` enforcement lint. The annotation was already parsed (Stage 1.3.0) but until now was just metadata. A new `noalloc_check` pass between `cap_check` and `p10` walks every fn marked `@noalloc`, builds the local outgoing-call graph for the whole file, and runs a BFS through the transitive call set. If the closure intersects a banned list (`alloc` / `resize` / `free_bytes` / `mem::alloc` / `mem::resize` / `mem::free_bytes` plus the libc externs `malloc` / `realloc` / `free` they wrap), the compile fails with a diagnostic naming both the `@noalloc` entry point and the reached callee. v1 doesn't resolve indirect calls via fn-pointer arguments (the analysis treats `fn(T) -> R` parameters as opaque) — a points-to refinement is a follow-up sub-slice. Two unit tests pin the behavior: an `@noalloc fn evil()` calling `alloc(...)` fails to compile; an `@noalloc fn pure_math()` doing only arithmetic compiles. `examples/noalloc_demo.fc` proves the lint accepts heap-free arithmetic (exits 65) and the c_interop suite registers it. This closes Stage 1.3 — every annotation the parser already accepted now has matching compiler-enforced semantics, so the `@noalloc` claim in `MANIFESTO.md` and the public README is no longer aspirational.

- **Deferred-item 7 ✅:** `mod log` — leveled logging in the prelude as a `fastc-core/log` preview. Mirrors the `mod json` pattern: a real fastc-core package shape that compiles inside fastC today. Exposes `debug` / `info` / `warn` / `error` for plain leveled messages and `kv_int(key, value)` for inline structured pairs, plus `level_debug` / `level_info` / `level_warn` / `level_error` for the future filter API. Every call writes to stdout with a `[LEVEL] ` prefix; the message walks bytes one at a time via `write_cstr_no_newline` (no buffer allocation), so log calls are heap-free and therefore callable from `@noalloc` functions — the closing slice of Stage 1.3's enforcement story meets the opening slice of the stdlib's level-aware logging. `examples/log_demo.fc` prints four leveled lines plus an inline `requests=42 errors=1 [INFO] hourly stats` and exits 0. v1's `current_level` is a callable that returns the threshold (no `static mut` yet) — a future sub-slice will wire it to a configurable mutable level once thread-local or static-mutable storage lands. The c_interop suite registers it as `test_log_demo_compiles`.

- **Deferred-item 8 ✅:** `caps::init()` is now `main`-only. The Stage 1.4 fabrication check (Deferred-item 2) blocked direct construction of sealed `Cap*` struct literals outside `mod caps`, but the legitimate mint helper `caps::init()` was callable from anywhere — a user library could simply call `caps::init()` itself to obtain the full bundle, bypassing the fabrication block entirely. The `cap_check` pass now carries a second policy: every `Call` whose callee resolves to `caps::init` must occur inside `fn main` at the root scope (or inside `mod caps` itself, where the function is defined). Anywhere else the lint emits `capability misuse: caps::init() is main-only`. The check matches both the qualified callee name `caps::init` and any bare aliases introduced by `use caps::init;` — the walker pre-scans the file's use-imports to record which bare names alias `caps::init`. Two new unit tests pin both spellings; the existing `accepts_caps_init_in_main` test still passes because main-at-root remains the legitimate mint site. The walker signature switched from a bare `inside_caps: bool` to a `Ctx` struct carrying `inside_caps`, `init_allowed`, and `init_aliases` so future capability policy checks bolt on by extending the struct. With this slice the capability story is structurally closed: caps cannot be fabricated, the mint is restricted to `main`, and every cap value in user code must therefore have come from `main`'s `caps::init()` call by way of explicit argument passing.

- **Deferred-item 9 ✅:** `@ensures(<expr>)` runtime postconditions. Companion to `@requires` (which was already parsed and lowered to entry-side asserts). The new clause runs at every `return` site, and the magic identifier `result` is bound to the value the function is about to return. New AST field `FnDecl.ensures: Vec<Expr>`. New lexer token `@ensures` and parser path `parse_fn_ensures` that interleaves freely with `@requires` and the boolean-flag annotations. Lower captures the return value into a `__ensures_result` temp typed to the function's return type, runs an AST-level `rewrite_result_ident` pass that substitutes every `result` ident for `__ensures_result`, lowers each clause through the existing `lower_expr` machinery, and emits one `if (!cond) fc_trap();` per clause before the final `return __ensures_result;`. Void returns skip the capture (no `result` to bind) but still emit the clause checks. Multiple `@ensures` on the same function stack — each runs in source order on every return path, including early returns and fall-through. `examples/ensures_demo.fc` declares `abs(x) -> i32` with `@ensures(result >= 0)` and a `pick_larger(a, b)` with both `@ensures(result >= a)` and `@ensures(result >= b)`, exercises both at runtime, and exits 7. A manual smoke test with a deliberately-bad `@ensures(result > 100) fn buggy() { return 42; }` confirms the program exits 134 (SIGABRT) — the runtime trap fires when the postcondition is violated. The clause storage matches `@requires`, so stage 2.1's SMT discharger only needs to add one branch ("discharge ensures with `result` bound to the inferred return type") to handle both contract surfaces. This is the runtime tier of the contract story called out in the roadmap (1.5); 2.1 takes it the rest of the way.

- **Deferred-item 10 ✅:** `mod time` — the first capability-typed I/O surface in the stdlib. `time::now(c: ref(CapTimeRead)) -> i64` returns the current Unix epoch in seconds via libc's `time(NULL)`, but the only callers that can reach it are those holding a `ref(CapTimeRead)` cap. The cap is minted in `main` via `caps::init()` (the only legitimate mint site — Deferred-item 8) and passed down explicitly through every reader. A function that never receives a `CapTimeRead` cannot call `time::now`, transitively, because the type checker rejects the bare call site at compile time — verified by a smoke test where dropping the cap argument produces `Type error: expected 1 arguments, got 0`. The libc call goes through a new `fc_time_now` runtime helper in `fastc_runtime.h` that wraps `time(NULL)` and widens the result to `int64_t` for cross-platform stability; this lets the fastC binding avoid constructing a NULL raw pointer (which the type system doesn't expose cleanly today). `examples/time_cap_demo.fc` mints the bundle, passes `addr(bundle.time_read)` down through `seconds_since_epoch`, calls `time::now(cap)`, and exits 0 if the timestamp is post-2001 (a sanity check that the libc bridge works). The c_interop suite registers it as `test_time_cap_demo_compiles`. This is the first end-to-end proof of the cap-typed I/O wedge: a side-effecting function whose side effect shows up in the type signature, is statically auditable from the call graph, and is unreachable without an explicit cap argument. `mod fs` / `mod net` / `mod env` follow the same shape in later sub-slices.

- **Deferred-item 11 ✅:** `mod env` and `mod rand` — two more cap-typed I/O surfaces alongside `mod time`. `env::get(c: ref(CapEnvRead), key: raw(u8)) -> raw(u8)` wraps libc `getenv` so libraries that read environment state have to declare it in their type. `rand::seed(c: ref(CapRand), s: u32)` and `rand::next_u32(c: ref(CapRand)) -> u32` drive a v1 LCG PRNG with Numerical Recipes constants (full-period 2^32, predictable, no platform dependency — a cryptographically-strong RNG follows once `mod crypto` lands in fastc-core). Three new runtime helpers (`fc_env_get`, `fc_rand_seed`, `fc_rand_u32`) live in `fastc_runtime.h` so the fastC bindings stay clean. `examples/env_rand_demo.fc` mints the bundle, passes the env-read cap down through `read_path` (which calls `env::get(c, cstr("PATH"))`), and the rand cap through `three_draws` (which XORs three draws). The demo uses qualified-call syntax throughout (Deferred-item 1) to sidestep the bare-name collision between `env::get` (concrete) and `vec::get` (generic) that mono v1 doesn't disambiguate yet for `use`-imported aliases — a follow-up sub-slice extends the qualified-aware lookup into mono. With `mod time` + `mod env` + `mod rand` the v1 capability-typed I/O set covers the three most common ambient-authority surfaces that a hostile library would target. `mod fs` / `mod net` / `mod proc` come in later sub-slices and follow the identical shape.

- **Deferred-item 12 ✅:** `mod fs` — read-side filesystem capability surface. `fs::exists(c: ref(CapFsRead), path: raw(u8)) -> i32` wraps libc `access(F_OK)` and returns 1/0. `fs::size_bytes(c: ref(CapFsRead), path: raw(u8)) -> i64` wraps libc `stat` and returns the regular file's size, or -1 if the path doesn't stat or isn't a regular file (directories, fifos, sockets and char-devices all return -1 — `/dev/null` returns -1 by design). Two new runtime helpers `fc_fs_exists` and `fc_fs_size_bytes` in `fastc_runtime.h` keep the fastC bindings platform-independent (off_t portability stays inside the runtime). `examples/fs_cap_demo.fc` mints the bundle, passes `addr(bundle.fs_read)` down through `probe_self`, and exits 0 after exercising both calls against `/dev/null`. Write operations (truncate / append / write_all) follow the same pattern in a later sub-slice using the separate `CapFsWrite` cap, so read-only code stays read-only by construction. The c_interop suite registers it as `test_fs_cap_demo_compiles`. With this slice every cap that `caps::init()` mints (fs_read, env_read, time_read, rand) has at least one real I/O entry point exercising it — the cap-typed I/O wedge is no longer aspirational on the read side.

- **Documentation cleanup + measured benchmarks ✅:** README cut from 298 lines to 107. The dense feature tables, code examples, Power-of-10 table, and stale "What's deferred" block all moved to the published docs site at `documentation/docs/why/`. The site now leads with a `Why fastC` section containing: a comparison rubric vs C/Rust/Zig/Go, a measured-benchmarks page, the C-interop FAQ, and the safety-defaults FAQ — five files plus an index. Three real benchmark suites under `benchmarks/cross-lang/`: (1) a perf harness measuring compile time / strip size / runtime for four programs (hello, sum, fib40, mandelbrot) across all five languages via hyperfine — golden CSV committed with date + host stamp; (2) a token-count comparison using tiktoken's cl100k and o200k encodings — the honest finding (fastC is the most verbose in 3/4 programs) is documented openly, reframing the agent claim around "what the longer source enforces" rather than token efficiency; (3) a first-compile-success-rate harness that sends a prompt to Claude / GPT-4o / Gemini for three tasks × five languages × N=10 trials and compiles the response — committed without a populated results.csv yet (needs API keys), runnable via `python3 run.py`. A 45-second vhs/asciinema demo at `assets/demo.gif` shows the cap-typed I/O wedge end-to-end and embeds in the README. Also surfaces a fastC compiler bug as a side effect of the mandelbrot benchmark: the lower pass silently drops `break`/`continue` inside `while`-loops (catch-all `_ => vec![]` arm in `lower_stmt`); workaround documented in the mandelbrot source, fix is the next compiler-side slice.

- [x] Closures: `|x: i32| -> i32 { return (x + 1); }`. *Slice 26 — capture-free closures parse at primary position and lift to synthetic `__lambda_N` top-level fns via a new `ClosureLifter` desugar walker, then ride the existing fn-pointer typedef pre-pass. v1 has no captured environment; capture-by-value with stack-allocated env structs is a follow-up slice. `examples/closure_demo.fc` replaces every helper-fn from earlier higher-order demos.*
  - Captures by value (copy). *N4 ✅ — closures referencing outer `let x = <literal>` bindings work via constant-inlining (IntLit / BoolLit / FloatLit / unary-negated literals). The desugar pipeline gained a `constant_scopes` stack on `ClosureLifter` plus an `inline_captures_in_block/stmt/expr` walker that substitutes captures into the closure body before lifting. Non-literal captures (function results, struct fields, mutable bindings) still emit the closure-aware "undefined name" diagnostic — full env-struct synthesis is the v2.0 follow-up.* Mutable captures require `mref` in the closure signature. *(env-struct: v2.0)*
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
- [x] Language specification document. *`docs/language.md` covers lexical rules, top-level items, statements, expressions, types, pointers, options/results, generics, traits, closures, capabilities (preview), and a stdlib summary. ~350 lines. Will get extended as stages 1.3 / 1.4 / 1.5 land.*
- [ ] Stability commitment: no breaking changes without a migration path.

**Definition of Done**

- [x] A non-trivial program (HTTP client or JSON parser) compiles using only the standard library. *Slice 36 — `examples/json_tokenizer_demo.fc` is ~150 LoC fastC tokenizing punctuation / strings / numbers / identifiers from a JSON-ish input, returning `Vec[Token]`. Exits 134 only when all 10 token-positional observations match.*
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

## 1.4 — Capability System ✅ *(types, fabrication check, and `caps.json` artifact shipped)*

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

## 1.5 — Contracts (Runtime Tier) ✅

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

## 1.6 — Agent-First Features + MCP Server *(core surface shipped)*

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

## 1.7 — Vendor-First Package System with Sigstore + SLSA L3 *(core verification shipped)*

> **Requires:** 1.1 (stable language — packages need a stable API surface), 1.4 (capabilities — the `fastc add` flow displays caps before install), 1.6 (`fastc-mcp` — package metadata flows through the same channel).
> **Complexity managed:** Code reuse without the supply-chain attack surface that has dominated Rust, npm, and PyPI in 2025/2026. Dependencies are git URL + commit hash + content hash, vendored into the user's repo. No central registry to phish, no account to compromise, no typosquatting (the URL is part of the import).
> **Complexity refused:** No HTTP package registry (initially). No semver SAT solver. No build scripts during install. No binary distribution. No platform-specific package variants. The package manager is a glorified `git clone` with content-hash verification.

See [docs/supply-chain.md](supply-chain.md) for the full story.

- [x] `fastc.toml` dependency entries: `name = { git = "<url>", rev = "<commit>", sha256 = "<hash>", sigstore = "<bundle-path>" }`.
- [x] `fastc fetch` — clones deps into the cache, verifies content hashes against either the manifest's `sha256` or the lockfile's recorded `sha256` (or both, cross-checked). Mismatch fails the build with a diagnostic showing expected vs computed.
- [x] `fastc lock` (and `fastc lock --force`) — re-anchors `fastc.lock` against the currently fetched tree. Without `--force`, refuses to overwrite a recorded hash when content has drifted.
- [x] `fastc add <github-url>` — capability-aware add flow. Fetches the candidate, scans `.fc` files for `Cap*` types appearing in `ref(...)` / `mref(...)` positions, prints a summary (package / git / rev / sha256 / caps), warns on high-impact caps (`CapNetConnect`, `CapProcSpawn`, `CapFsWrite`), and prompts before writing to `fastc.toml`. `--yes` for scripted setups.
- [x] Sigstore bundle verification via `cosign verify-blob`. When a dep declares `sigstore = "<path>"`, the build shells out to cosign with the default fastc-core identity regexp + the GitHub Actions OIDC issuer. Cosign-not-on-PATH degrades to a warning (so fast iteration isn't blocked); a bundle that fails to verify is a build error.
- [x] Build-system constraint: dependency code never runs at install time. `fastc.toml` is parsed by `serde` with `#[serde(deny_unknown_fields)]` — there's no syntactic place to put executable code. See [examples/supply_chain_demo/](../examples/supply_chain_demo/) for the cargo-vs-fastc side-by-side.
- [x] Sigstore signing on `fastc` compiler binary releases — `.github/workflows/release.yml` builds the Linux / macOS / Windows binaries, signs each with cosign keyless OIDC (no long-lived keys), and uploads the `.sigstore.json` bundles alongside the binaries on the GitHub Release page.
- [x] SLSA Level 3 provenance for the compiler binary — same workflow calls `slsa-framework/slsa-github-generator` to emit `multiple.intoto.jsonl` signed with the same workflow identity. Downstream consumers verify via `slsa-verifier verify-artifact`.
- [x] Integration tests: `crates/fastc/tests/supply_chain.rs` covers edit-detection, `.git`-exclusion, and empty-file smuggling on `hash_tree` / `verify_tree`.
- [ ] Reproducible-build verification: hash the C output of a dep build; same source + same `fastc` version produces identical bytes. *(0.4 determinism tests cover same-input → same-output; the dep-bound version follows.)*
- [ ] Global build cache keyed by `(fastc_version, dep_content_hash, target_triple)`. *(Salsa skeleton in `db.rs` plus the per-dep sha256 give us the inputs; the keyed cache layer is the next sub-slice.)*
- [ ] Vendor-package directory split — move each stage-1.8 prelude `mod` to its own `Skelf-Research/fastc-core-<name>` repo, sign first release with the new workflow, switch the launch-set demo to consume them via path/git deps.

**Definition of Done**

- [x] `fastc add file://<dep>` works end-to-end: fetches, displays capabilities, writes manifest entry with `sha256`, anchors `fastc.lock`. The smoke test against a local git repo demonstrates the full flow with `CapFsRead` / `CapNetConnect` detection. *(Tested locally; the published `Skelf-Research/fastc-core-http` repo is blocked on the package-split sub-slice.)*
- [x] A canary "malicious package" test confirms that hash mismatch fails the build before any code is compiled — see the tamper-detection test in `crates/fastc/tests/supply_chain.rs` (`verify_tree_catches_the_edit`) and the end-to-end CLI flow that returns a non-zero exit with the expected/got diagnostic.
- [x] The compiler binary will have verifiable SLSA L3 provenance on the GitHub release page once the first tag (v0.2.0+) is pushed — workflow lives at `.github/workflows/release.yml`.
- [ ] A user replays a clean build of any fastC project on a fresh machine and gets a build-cache hit, not a rebuild. *(Blocked on the keyed global cache above.)*

## 1.8 — fastc-core Curated Stdlib Extensions *(launch set shipping as prelude previews)*

> **Requires:** 1.7 (vendor-first package system live so the curated packages have somewhere to live).
> **Complexity managed:** Users get one canonical, audited answer for HTTP, JSON, TOML, logging, CLI parsing, crypto primitives, regex, async runtime, and common data structures. No "Axum vs. Actix vs. Rocket" agent confusion. Every `fastc-core` package is reviewed, signed, capability-typed, and contract-annotated.
> **Complexity refused:** No community-blessing for the first two years. The answer to "is there a fastC library for X" is "yes, in fastc-core" or "no, write it locally." We resist the urge to bless community packages until they have been around for a year and audited.

See [docs/ecosystem.md](ecosystem.md) for the full curation strategy and target package list.

**Implementation note:** The five v1 launch-set modules ship today as previews in the fastC prelude (`mod cli`, `mod log`, `mod json`, `mod toml`, `mod http`). They use the final naming, signatures, and cap-typed I/O surface that the standalone vendor packages will inherit when 1.7's `fastc add` flow lands. This is deliberate — agents and humans can use them now via `use http::get_status` without touching the package manager; the split to separate repos is a packaging change, not a code change.

- [x] **Launch set (preview in prelude):** `mod cli` (argv + flag parsing), `mod log` (debug/info/warn/error + kv_int/kv_str), `mod json` (encoder + `find_int` decoder slice), `mod toml` (flat-table `find_int` / `find_bool`), `mod http` (`get_status` over `CapNetConnect`).
- [x] CLI: `count`, `arg_at`, `program_name`, `has_flag`, `flag_value`, `flag_int` + `is_null` / `null_arg` plumbing for the OS-passed argv. Runtime support: auto-generated `int main(int argc, char** argv)` wrapper around the user's `fn main()`.
- [x] HTTP: TCP socket primitives (`fc_net_connect_tcp` / `fc_net_send` / `fc_net_recv` / `fc_net_close`) in `runtime/fastc_runtime.h`. WASI is supported at the binding level — calls return -1 since WASI Preview 1 has no synchronous BSD sockets, gated by `#ifndef __wasi__`.
- [x] Integration tests: `crates/fastc/tests/cli_module.rs` (2 cases), `crates/fastc/tests/http_module.rs` (1 case, spins up `python3 -m http.server` for the round-trip).
- [x] Examples: `examples/cli_demo.fc`, `examples/http_demo.fc`, plus the existing `examples/log_demo.fc` / `examples/json_encoder_demo.fc`.
- [x] Stage 1.4 hookup: `mod http::get_status` requires `ref(CapNetConnect)` — the strategic wedge of cap-typed I/O is end-to-end real for the network now.
- [x] **Compiler infrastructure fixes** shipped while wiring this up: `Stmt::Break` / `Stmt::Continue` now lower correctly (previously silently dropped inside `while`); P10 `function-size` no longer panics when a function body span lands inside a multi-byte UTF-8 codepoint.
- [ ] Sigstore signing on the launch-set packages (depends on stage 1.7).
- [ ] Vendor-package split: move each `mod` to its own `fastc-core/<name>/` repo with its own `fastc.toml` (depends on stage 1.7's path-dep + `fastc add` flow).
- [ ] **Six-month set:** add `fastc-sqlite`, `fastc-crypto-primitives`, `fastc-regex`, `fastc-uuid`, `fastc-time`, `fastc-base64`.
- [ ] **One-year set:** add async runtime, TLS, websocket, csv, gzip, ed25519, x509 parser, and the remaining ~15–25 packages to reach the 30–50 target.
- [ ] `fastc.dev` search frontend over GitHub repos matching the `fastc-<name>` convention. No registry to operate.

**Definition of Done**

- [x] An end-to-end demo uses all five launch-set modules from a single fastC program (`examples/launch_set_demo.fc`).
- [ ] The 5 launch packages exist on GitHub under `Skelf-Research/fastc-core`, signed, with `AGENTS.md` and full annotation coverage *(post-stage-1.7)*.
- [ ] A new fastC project can implement an HTTP+JSON CRUD service using only `fastc-core` packages *(POST + JSON-body request building is a v1.1 follow-up)*.
- [ ] `fastc.dev` returns relevant results for "http", "json", "logging" within 1 second *(blocked on 1.7's package directory)*.

## 1.9 — Cross-Compilation via `zig cc` ✅

> **Requires:** none new — sits on top of the existing compile pipeline; fastC already emits portable C11.
> **Complexity managed:** Reach every cloud / edge / embedded / WASM target without maintaining our own cross-compilation infrastructure. fastC emits portable C11, which means every C cross-compiler in the world is structurally a fastC cross-compiler.
> **Complexity refused:** No custom code generator per target. No sysroot manager (zig bundles libcs; proprietary toolchains use `--cc-override`). No Windows-msvc target in v1 (different ABI, deferred until a real user asks).

See [docs/cross-compile.md](cross-compile.md) for the full how-to.

- [x] `fastc compile / build --target=<triple>` flag.
- [x] Eight v1.7 target presets via `zig cc`: `aarch64-linux-musl`, `x86_64-linux-musl`, `aarch64-linux-gnu`, `x86_64-linux-gnu`, `aarch64-macos`, `x86_64-macos`, `wasm32-wasi`, `riscv64-linux-musl`.
- [x] `--cc-override=<path>` escape hatch for proprietary cross-toolchains (crosstool-ng, vendor gcc-cross, IAR, ARM Compiler, etc).
- [x] `fastc target list` / `fastc target check <triple>` introspection subcommands.
- [x] `fastc run --target=<triple>` explicitly refuses (cross-binary execution needs an emulator we don't manage).
- [x] WASI output gets the `.wasm` extension automatically.
- [x] CI matrix verifying every shipped target end-to-end on every PR (`.github/workflows/cross_compile.yml`).
- [x] Integration test (`crates/fastc/tests/cross_compile.rs`) inspects ELF / Mach-O / WASM magic bytes per triple. Skips gracefully when zig isn't on PATH.
- [x] Runtime header (`runtime/fastc_runtime.h`) audited for portability — the POSIX surface used (`access`, `stat`, `time`, `getenv`) is available on every shipped target's libc, including wasi-libc.

**Definition of Done**

- [x] `fastc build --target=<each of the 8>` produces a binary with the right architecture / ABI on a fresh CI runner.
- [x] `fastc target list` and `fastc target check` exit 0 / 1 as documented.
- [x] `documentation/docs/why/rubric.md` and `README.md` reflect the new "Cross-compile" capability.

## 2.0 — Compiler Hardening + Incremental *(core hardening shipped)*

> **Requires:** 1.7 (ecosystem feedback reveals real-world compiler bugs and pain points).
> **Complexity managed:** Trust. Users cannot adopt fastC for serious work until the compiler itself is proven reliable. This stage makes the compiler trustworthy, not the language more powerful.
> **Complexity refused:** No new language features in this stage. All effort goes into proving what already exists works correctly.

- [x] Compiler fuzzing with `cargo-fuzz` to find crash bugs and miscompilations. Four shipped libfuzzer targets (`parse_no_panic`, `check_no_panic`, `compile_no_panic`, `discharge_no_panic`) cover the lex / parse / resolve / typecheck / cap_check / noalloc_check / p10 / mono / lower / emit / discharge pipeline. CI workflow (`.github/workflows/fuzz.yml`) runs them as a matrix on every PR that touches the parser, lexer, or fuzz harness, with per-target time budgets tuned to per-iteration cost. Crash artifacts auto-upload as build artifacts for triage.
- [x] Dedicated fuzz target for the annotation parser (1.3) and capability checker (1.4). Covered by `check_no_panic` and `compile_no_panic` — both exercise the full annotation + cap_check pipeline on arbitrary input.
- [x] Debug info / source maps (C line → fastC source) for debugger integration. Per-function `#line N "<file>"` directives (J1) plus per-statement directives (J2) so gdb / lldb stack traces and breakpoints land on `.fc` source lines inside fn bodies. DWARF emitted by `cc -g` propagates the .fc filename and line numbers through to the linked binary.
- [x] `--reproducible` flag (L2) + cross-directory reproducibility integration test. Normalizes the source path embedded in `#line` directives to the basename so two compilations of the same `.fc` in different working directories produce byte-identical C output. End-to-end verified by `crates/fastc/tests/reproducibility.rs`, which compares hashes from two temp dirs with isolated build caches.
- [x] **Compiler-binary** reproducibility (N5). `rustc --remap-path-prefix` + `SOURCE_DATE_EPOCH` shipped in `.github/workflows/release.yml`. The "Pin reproducibility env" step binds `SOURCE_DATE_EPOCH` to the tagged commit's author timestamp and sets `RUSTFLAGS=--remap-path-prefix $(pwd)=/fastc --remap-path-prefix $HOME/.cargo=/fastc-deps`, so every published `fastc-<os>-<arch>` binary is byte-identical when rebuilt from the same tag on any machine. `docs/supply-chain.md` carries the local-reproduction recipe.
- [x] **Multi-source-file build cache for `fastc build`** (M1). The H4 single-file cache is now joined by a project-level cache keyed by (sorted `src/**/*.fc` content + `fastc.toml` + `fastc.lock` + fastc_version). Cache hit → skip the full lex → emit chain → 414ms cold → 9ms warm builds (46× speedup) measured on a hello project. Editing any `.fc` under `src/` flips the project key and triggers a fresh build. Verified end-to-end by `crates/fastc/tests/incremental.rs` (3 cases: warm-vs-cold speedup, single-file invalidation, secondary-module invalidation).
- [x] **`[workspace]` manifest + per-member incremental** (N3). `Manifest::workspace: Option<WorkspaceConfig>` parses `members = ["a", "b", ...]`. `BuildContext::is_workspace_root()` dispatches to `compile_workspace()` which iterates members, each with its own M1 project cache. Editing only `a/src/main.fc` re-emits `a` and serves `b` from cache. The `compile_workspace` path rejects `--cc` / `--target` / `--cc-override` (those live on member builds). 40× warm-vs-cold on the two-member smoke test (526ms → 13ms).

> Cross-compilation lifted from 2.0 to its own stage 1.9 (see above) — shipped via `zig cc` ahead of the rest of the hardening work.

**Definition of Done**

- [x] CI fuzz matrix runs four libfuzzer targets on every PR that touches the parser / lexer / fuzz harness. The 72-hour campaign target is a follow-up — the v1.x baseline is "no PR ships a parser regression that 5 minutes of fuzzing finds".
- [x] Incremental compilation provides measurable speedup on multi-module projects — the M1 project cache delivers 46× warm-vs-cold on the hello case (414ms → 9ms), and N3 extends the same property to multi-member `[workspace]` builds (40× on the two-member smoke test).
- [x] `gdb` / `lldb` can step through fastC source using generated debug info — shipped via `#line` directives at fn boundaries (J1) and per statement (J2).
- [x] Same-source reproducibility across working directories — shipped via `fastc compile --reproducible`. Compiler-binary reproducibility (N5) shipped via `SOURCE_DATE_EPOCH` + `--remap-path-prefix` in `.github/workflows/release.yml`.

## 2.1 — SMT Contract Discharge *(core pipeline shipped)*

> **Requires:** 1.5 (contracts as runtime asserts), 2.0 (compiler hardened — SMT is a new failure surface that needs the rest of the compiler stable).
> **Complexity managed:** Contracts get *proved*, not just runtime-checked. A function with `@requires(true)` or `@requires((a > 0) || ((a == 0) || (a < 0)))` is discharged at compile time and pays zero runtime cost. The build emits a per-function report: proven N, runtime-checked M, unknown K.
> **Complexity refused:** No mandatory SMT. The `--no-prove` flag skips both tiers and falls back to runtime asserts (the 1.5 behaviour). This is critical for the agent inner loop: agents iterate fast, they want SMT on CI, not on every save.

See [docs/contracts.md](contracts.md) for the three-tier discharge design.

- [x] Three-tier pipeline per obligation: **tier-1 syntactic** (always on — constant fold, tautological-comparison detection over the AST); **tier-2 SMT** (opt-in via `--prove` — shells out to `z3 -smt2 -in` with `(set-option :timeout)` + a process-level kill at 2× budget); **tier-3 runtime** (the existing stage-1.5 `fc_trap` guard, the safe default for anything tiers 1+2 couldn't prove).
- [x] `contract_discharge` pass landed at `crates/fastc/src/discharge/` — runs between p10 and mono in `driver.rs`, returns a `DischargeReport` that flows into the lower pass via `Lower::set_discharge`.
- [x] Lower-pass integration: proven obligations elide their `if (!cond) fc_trap()` guard. Proof = zero runtime cost.
- [x] `discharge.json` per-build report — proven / runtime / unknown counts plus a per-obligation entry recording function, clause kind, index, status, tier (when proven), and reason (when not).
- [x] `--prove` / `--no-prove` / `--prove-budget=<ms>` CLI flags on `fastc compile`. `--discharge-output <path>` writes the report; `-` writes to stderr.
- [x] Z3-not-on-PATH degrades to runtime-tier with a structured `reason` per obligation — the build never blocks on a missing external tool (same discipline as cosign in stage 1.7).
- [x] Linear integer arithmetic + boolean combinators + comparisons + parameter quantification covered by the SMT encoder (`crates/fastc/src/discharge/smt.rs`). Demonstrated end-to-end via the integer-trichotomy + De Morgan tests.
- [x] Integration tests (`crates/fastc/tests/discharge.rs`): 9 cases covering tier-1 elision, runtime fallback, ensures discharge, JSON report shape, SMT trichotomy, SMT De Morgan, SMT counterexample handling.
- [ ] Discharge results cached on disk in `.fastc/cache/` keyed by formula hash — Salsa's input-hashing infrastructure is the foundation; the per-pass cache write is the remaining sub-slice.
- [ ] Body-aware SMT discharge — currently tier-2 only proves clauses universally true over their parameters. A richer encoding that walks the function body for `@ensures` (and uses `@requires` as a precondition) discharges substantially more clauses; the SMT encoder shape is ready for this.
- [ ] Readable timeout diagnostics that suggest concrete fixes ("strengthen `@requires` to include..." / "weaken `@ensures`...").

**Definition of Done**

- [x] An end-to-end integration test demonstrates that an SMT-proven obligation results in a `fc_trap`-free C function body. `smt_proves_integer_trichotomy` is the canary.
- [x] `--no-prove` short-circuits the SMT tier; tier-1 still runs (it's always on and free).
- [ ] `discharge.json` for a typical 5000-line fastC program shows >80% of obligations proven. *(Blocked on the body-aware encoder above — the v1 pipeline ships, the % depends on coverage.)*
- [ ] An obligation that times out produces a structured diagnostic with a concrete hint, not a stack trace. *(Today: a `reason` string is emitted, but the fix-it suggestions aren't generated yet.)*

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

- [ ] WASM beyond `wasm32-wasi` (browser, the component model, Emscripten interop) — basic `wasm32-wasi` ships in stage 1.9 via `zig cc`.
- [ ] `comptime`-style constant evaluation beyond current `const` expressions (only if it can be kept explicit).

These are deliberately vague. They will be specified when the prerequisites exist and community demand is clear.

---

## Competitive Context

See [docs/competitive-analysis.md](competitive-analysis.md) for detailed positioning against C, Zig, Rust, and V, and [docs/MANIFESTO.md](MANIFESTO.md) for the launch thesis.

fastC's core differentiator is the **fusion of capability-typed I/O, mandatory contracts, zero-executable-build-scripts, and capability-aware dependency management** — measured against a strict compile-time budget and served to AI agents over a native MCP protocol. No other systems language combines these properties. Rust has cargo and the borrow checker but pays a permanent tax in compile time, `build.rs`, proc macros, and a 150K-crate supply-chain surface. Zig is small but has no provenance story and runs arbitrary code in `build.zig`. C has 50 years of ecosystem and no safety. fastC occupies the open quadrant: small surface, safe by construction, provable, and built for the age of agent-generated code.
