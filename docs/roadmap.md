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
- [ ] Integrate module resolution into compilation pipeline.
- [ ] Wire up dependency fetching in build command.

**Definition of Done**

- [x] Tutorial examples cover all major language features.
- [x] Advanced examples demonstrate real-world patterns (FFI, networking, algorithms).
- [x] `fastc new my_project` creates a working project structure.
- [ ] Module imports work across files.
- [ ] Dependencies can be fetched from Git URLs with version pinning.

---

## Design Principle: Managing Complexity

FastC's [design principles](overview.md) — clarity over cleverness, explicit semantics, safe by default, local reasoning, explicit effects — are not just philosophy. They are **constraints that every future feature must satisfy.** A feature that violates these principles does not ship, no matter how popular it is elsewhere.

This means every stage in the roadmap must answer two questions:

1. **What complexity does this stage manage?** (What problem becomes tractable that wasn't before?)
2. **What complexity does this stage refuse to introduce?** (What simpler alternative did we choose over the "industry standard" approach?)

### Dependency Chain

Each stage builds on the previous. Nothing is standalone.

```
0.7 Modules ─────────► Programs can span multiple files
    │                   (requires: module resolution, #include generation)
    ▼
0.8 Generics ────────► Data structures can be type-safe without duplication
    │                   (requires: modules for multi-file generic code)
    ▼
0.9 Traits ──────────► Abstraction without runtime overhead
    │                   (requires: generics for bounded polymorphism)
    ▼
1.0 Closures + Stdlib ► Non-trivial programs without C escape hatch
    │                   (requires: traits for iterators, Drop for cleanup)
    ├──────────────────►
    ▼                   ▼
1.1 Benchmarks       1.2 Agent Features
    │                   │
    ▼                   ▼
    Real programs exist to benchmark and to test agent workflows
    │
    ▼
1.5 Packages ────────► Ecosystem: code reuse across projects
    │                   (requires: stable language from 1.0)
    ▼
2.0 Hardening ───────► Compiler is trustworthy (fuzzing, debug info)
    │                   (requires: real-world feedback from ecosystem)
    ▼
2.1 Certification ───► Safety-critical deployment evidence
    │                   (requires: hardened compiler)
    ▼
2.2 Effects ─────────► Compile-time proof of purity, no-alloc, no-diverge
    │                   (requires: traits for effect bounds, certification feedback)
    ▼
2.3 Async ───────────► Explicit coroutines via Future[T] trait
                        (requires: closures, Drop, benefits from effect system)
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

**What 1.2 extends:**
- JSON output from `cert-report` only → all commands (`compile`, `check`, `fmt`)
- Fix-it hints from display-only → auto-applicable via `fastc fix`
- Type surface from LSP-only → exportable via `fastc context`
- Diagnostics from single-file → project-wide with cross-module spans

This is the key insight: **the compiler's constraints are not limitations — they are the API surface for tooling.** Every rule the compiler enforces is a rule that tooling can report on, fix automatically, and verify programmatically.

---

## 0.7 — Foundation Completion

> **Requires:** 0.6 (module parsing, manifest infrastructure).
> **Complexity managed:** Programs can span multiple files without copy-pasting code or relying on C `#include` hacks.
> **Complexity refused:** No complex module visibility rules. Modules are files. `pub` means visible outside the module. That's it.

- [ ] Wire module resolution into name resolver (`resolve/mod.rs` currently flattens modules).
- [ ] Wire dependency fetching into `fastc build` (build.rs has infrastructure but doesn't feed into pipeline).
- [ ] Implement `use` path resolution in type checker (`typecheck/mod.rs` skips `Item::Use`).
- [ ] Multi-file C output with correct `#include` relationships.
- [ ] GitHub Actions CI (`cargo test` + C compilation with gcc/clang).
- [ ] Cross-platform CI (Linux x86_64, macOS ARM64).

**Definition of Done**

- [ ] `mod utils;` + `use utils::helper;` compiles to working C11.
- [ ] CI runs green on every push and PR.
- [ ] Dependencies from `fastc.toml` are fetched and compiled.

## 0.8 — Generics via Monomorphization

> **Requires:** 0.7 (modules — generic code must work across files).
> **Complexity managed:** Type-safe data structures without code duplication. `vec(i32)` and `vec(f64)` share one definition, generate separate C code.
> **Complexity refused:** No type erasure, no vtables, no runtime generics. Monomorphization means every generic instantiation is fully resolved at compile time — the C output contains no `void*` casts, no indirection. This preserves local reasoning: you can read the generated C and understand exactly what runs.

- [ ] Grammar extension: `fn find_min[T](s: slice(T), len: i32) -> T`.
- [ ] Type parameter parsing and AST representation.
- [ ] Monomorphization pass between type checking and lowering.
- [ ] Generic structs: `struct Pair[A, B] { first: A, second: B }`.
- [ ] Generic function instantiation with concrete types.
- [ ] Minimal constraint system (`T: Eq`, `T: Ord`) as a stepping stone to traits.
- [ ] Error diagnostics for unsatisfied constraints.

**Definition of Done**

- [ ] Generic functions and structs work end-to-end.
- [ ] Monomorphization generates specialized C functions (e.g., `find_min_i32`, `find_min_f64`).
- [ ] Constraints are checked at call sites with clear error messages.

## 0.9 — Traits and Method Syntax

> **Requires:** 0.8 (generics — traits bound generic type parameters).
> **Complexity managed:** Abstraction without runtime cost. A function constrained by `T: Ord` can compare values without knowing the concrete type at the call site, but the generated C is still a direct function call — no vtable lookup, no dynamic dispatch.
> **Complexity refused:** No trait objects (`dyn Trait`). All dispatch is static. This is a deliberate trade-off: you cannot store heterogeneous types in a collection via traits. But you always know exactly which function is called, and the C output proves it. If dynamic dispatch is needed, use explicit function pointers in an `unsafe` block.

- [ ] Trait declarations: `trait Eq { fn eq(self: ref(Self), other: ref(Self)) -> bool; }`.
- [ ] Trait implementations: `impl Eq for Point { ... }`.
- [ ] Trait bounds on generic parameters: `fn max[T: Ord](a: T, b: T) -> T`.
- [ ] Built-in traits: `Eq`, `Ord`, `Copy`, `Drop`.
- [ ] Method call syntax: `x.method(args)` desugars to static dispatch.
- [ ] Compiler-generated `Drop` calls at scope exits for types implementing `Drop`.

**Definition of Done**

- [ ] Trait-bounded generics compile to static dispatch C.
- [ ] Method syntax works on types with trait implementations.
- [ ] `Drop` trait enables deterministic resource cleanup.

## 1.0 — Standard Library and Closures (MVP)

> **Requires:** 0.9 (traits for iterators and Drop, generics for containers).
> **Complexity managed:** Self-sufficient programs. After 1.0, a user can write a non-trivial program without escaping to C. The standard library is written in FastC itself — proving the language is expressive enough.
> **Complexity refused:** No implicit memory management. `vec` and `hashmap` allocate explicitly and clean up via `Drop`. No garbage collector, no reference counting by default. The programmer sees every allocation because the stdlib calls `fc_alloc` / `fc_free` through the `mem` module. Closures capture by explicit value copy, not by hidden reference — no closure lifetime puzzles.

- [ ] Closures: `|x: i32| -> i32 { return (x + 1); }` lowered to C structs with captured environment.
  - Captures are by value (copy). Mutable captures require `mref` in the closure signature.
  - No implicit heap allocation for closures — they are stack-allocated structs.
- [ ] Standard library written in FastC:
  - [ ] `io` — file I/O, stdin/stdout
  - [ ] `string` — owned strings, slicing, formatting
  - [ ] `vec` — growable array (generic, requires 0.8)
  - [ ] `hashmap` — hash table (generic, requires 0.8 + `Eq` trait from 0.9)
  - [ ] `mem` — allocators, copy, move
  - [ ] `math` — numeric functions
  - [ ] `fs` — filesystem operations
- [ ] Iterator protocol via traits + closures.
- [ ] Doc comments (`///`) parsed and available to tooling.
- [ ] Language specification document.
- [ ] Stability commitment: no breaking changes without a migration path.

**Definition of Done**

- [ ] A non-trivial program (HTTP client or JSON parser) compiles using only the standard library.
- [ ] Standard library has test coverage and documentation.
- [ ] Language specification is published.

## 1.1 — Benchmarking Infrastructure

> **Requires:** 1.0 (real programs to benchmark — toy benchmarks are meaningless).
> **Complexity managed:** Honest performance data. Without benchmarks, claims about "C-like performance" are hand-waving. With benchmarks, we know exactly where safety checks cost performance and by how much.
> **Complexity refused:** No benchmark-driven optimization. We do not add compiler special-cases to win benchmarks. If bounds checks cost 3% on n-body, we report 3% — and explain why that trade-off is worth it.

Establish a rigorous, reproducible benchmarking framework. See [docs/benchmarking.md](benchmarking.md) for full methodology.

- [ ] `bench/` directory with cross-language benchmark suite.
- [ ] 6 CLBG-style programs: n-body, binary-trees, spectral-norm, mandelbrot, fannkuch-redux, fasta.
- [ ] Micro-benchmarks: array-sum, struct-access, bounds-check overhead, ffi-call.
- [ ] Custom harness: shell/Python orchestrator using `hyperfine` + `perf`.
- [ ] Agent usability benchmarks (error recovery rate, code gen accuracy, diagnostic parsability).
- [ ] Compile-time benchmarks comparing `fastc+cc` vs `gcc` vs `clang` vs `zig` vs `rustc`.

**Definition of Done**

- [ ] `./bench/run_all.sh` produces reproducible markdown comparison tables.
- [ ] Benchmarks run in CI with historical tracking.
- [ ] Results are published with hardware specifications and methodology notes.

## 1.2 — Agent-First Features

> **Requires:** 1.0 (agents need a real language to work with — agent tooling for a toy language proves nothing).
> **Complexity managed:** The gap between "compiler says there's an error" and "the error is fixed." Today, a human reads the error, understands it, and edits the code. Agent features close that loop automatically: `check → fix → check` converges to working code.
> **Complexity refused:** No AI inside the compiler. `fastc fix` applies deterministic fix-it hints, not LLM suggestions. The compiler remains a pure function from source to output. Agent intelligence lives in the agent, not in the toolchain.

Make FastC the best language for AI coding agents. See [docs/agent-features.md](agent-features.md) for full specification.

This builds on existing infrastructure: `cert-report` already supports `--format json|compact|text`, miette diagnostics already carry spans and fix-it hints, and P10 violations already produce structured `ViolationDetail` records. The work here extends that foundation to all compiler commands.

- [ ] Extend `--output-format=json` from `cert-report` to all CLI commands (`compile`, `check`, `fmt`).
- [ ] `fastc fix` command — auto-apply the existing `.with_help()` fix-it hints from diagnostics.
- [ ] `fastc context` — dump project type surface for AI context windows (leverages the type checker's resolved symbol table).
- [ ] `fastc diff` — semantic code diff (AST-level, not text-level).
- [ ] Inline `test { }` blocks compiled only in test mode.
- [ ] LSP enhancements: code actions (from fix-it hints), semantic tokens, workspace rename.
- [ ] Unify `CompileError` diagnostics and `P10Violation` reports into a single JSON diagnostic stream.

**Definition of Done**

- [ ] An agent can iterate `check → fix → check` to reach working code without human intervention.
- [ ] `fastc context` output fits in a typical LLM context window and captures all public API surfaces.
- [ ] All CLI output is machine-parseable when `--output-format=json` is passed.
- [ ] JSON diagnostic format includes compiler errors, safety violations, and P10 compliance in one stream.

## 1.5 — Package Registry and Ecosystem

> **Requires:** 1.0 (stable language — packages need a stable API surface to depend on).
> **Complexity managed:** Code reuse without copy-paste. A JSON parser should be written once, tested once, and used by everyone.
> **Complexity refused:** No complex dependency resolution (no SAT solvers). Semver with a simple "newest compatible" resolver. No build scripts that execute arbitrary code during `fastc add`. Packages are FastC source, compiled by the same `fastc` pipeline — no binary distribution, no pre-built artifacts, no platform-specific package variants.

- [ ] HTTP-based package registry with JSON index.
- [ ] `fastc publish` — publish packages to the registry.
- [ ] `fastc add <package>` — add a dependency and update `fastc.toml`.
- [ ] Semver resolution for dependency versions.
- [ ] 10–20 seed packages: `json`, `http`, `csv`, `cli`, `log`, `test`, `regex`, `crypto`, `toml`, `yaml`.
- [ ] Package documentation generation from doc comments.

**Definition of Done**

- [ ] `fastc add json` works end-to-end: resolves version, downloads, adds to `fastc.toml`, compiles.
- [ ] Registry serves package metadata and tarballs over HTTPS.
- [ ] Seed packages have documentation and tests.

## 2.0 — Compiler Hardening

> **Requires:** 1.5 (ecosystem feedback reveals real-world compiler bugs and pain points).
> **Complexity managed:** Trust. Users cannot adopt FastC for serious work until the compiler itself is proven reliable. This stage makes the compiler trustworthy, not the language more powerful.
> **Complexity refused:** No new language features in this stage. All effort goes into proving what already exists works correctly.

- [ ] Compiler fuzzing with `cargo-fuzz` to find crash bugs and miscompilations.
- [ ] Debug info / source maps (C line → FastC source) for debugger integration.
- [ ] Incremental compilation (only recompile changed modules and their dependents).
- [ ] Cross-compilation support (target triples, sysroot configuration).

**Definition of Done**

- [ ] Compiler passes 72-hour fuzzing run with no crashes or miscompilations.
- [ ] Incremental compilation provides measurable speedup (>2x) on projects with 10+ modules.
- [ ] `gdb` / `lldb` can step through FastC source using generated debug info.

## 2.1 — Safety-Critical Certification

> **Requires:** 2.0 (compiler hardening — certification bodies require evidence of compiler reliability).
> **Complexity managed:** Regulatory compliance. FastC's transpilation model is a genuine advantage here: certify the C output with an already-qualified C compiler, rather than qualifying an entire new compiler backend.
> **Complexity refused:** FastC does not become a "certification framework." It produces evidence (traceability reports, P10 compliance data, test coverage metrics) that feeds into existing DO-178C / IEC 62304 / ISO 26262 processes. The certification workflow is the user's responsibility — FastC provides the artifacts.

- [ ] DO-178C / IEC 62304 certification evidence package.
- [ ] Traceability: FastC source line → C output line → binary instruction.
- [ ] P10 compliance reports integrated into certification artifacts.
- [ ] Formal verification integration (CBMC / Frama-C on emitted C11).

**Definition of Done**

- [ ] A reference project (e.g., flight controller or medical device driver) passes certification review using FastC-generated evidence.
- [ ] Formal verification can prove absence of runtime errors on a 500-line FastC program.

## 2.2 — Effect System

> **Requires:** 0.9 (traits for effect bounds), 2.1 (certification feedback reveals which effects matter in practice).
> **Complexity managed:** Knowing what a function *does* — not just what it returns. Today, you can call any function from any context and only discover at runtime that it allocates, does I/O, or diverges. An effect system makes these properties checkable at compile time. This directly serves three goals:
> - **Safety-critical certification:** "This function is verified `@noalloc` and `@nodiverg`" is exactly what DO-178C auditors want.
> - **Agent usability:** `fastc context` can show effect annotations — an agent knows a `@pure` function has no side effects without reading the body.
> - **Async foundation:** Async is an effect. If the effect system exists, `async` becomes a principled extension, not a bolted-on feature.
>
> **Complexity refused:** No algebraic effects (hidden control flow via effect handlers), no monadic effects (Haskell-style, too abstract for a C-like language). FastC effects are *checked annotations*, not a computation model.

FastC already partially tracks effects through P10 rules (P10-003 restricts dynamic allocation, P10-001 restricts recursion). The effect system generalizes these into the type system.

**Design: effects as checked annotations, not a type-level computation.**

```fastc
// Declare effects on function signatures
@pure
fn add(a: i32, b: i32) -> i32 {
    return (a + b);
}

@noalloc
fn process(buf: slice(u8), len: i32) -> i32 {
    // Error: vec_push allocates — violates @noalloc
    // let v: vec(u8) = vec_new();
    return cast(i32, at(buf, 0));
}

@nodiverg
fn bounded_loop(n: i32) -> i32 {
    let sum: i32 = 0;
    for (let i: i32 = 0; (i < n); i = (i + 1)) {
        sum = (sum + i);
    }
    return sum;
}
```

**Effect hierarchy:**

| Effect | Guarantees | Subsumes |
|--------|-----------|----------|
| `@pure` | No I/O, no allocation, no mutation of external state, no divergence | `@noalloc` + `@noio` + `@nodiverg` |
| `@noalloc` | No heap allocation (`fc_alloc` / `fc_free` never called) | — |
| `@noio` | No file/network/stdio operations | — |
| `@nodiverg` | Always terminates (no unbounded loops, no recursion) | — |

**Checking rules (local, per-function):**

- A function marked `@noalloc` can only call functions that are also `@noalloc` (or `@pure`).
- A function marked `@pure` can only call other `@pure` functions.
- Violations are compile-time errors with fix-it hints: "remove `@noalloc` annotation" or "mark called function as `@noalloc`".
- Effects are opt-in. Unannotated functions are unconstrained — they may do anything. This avoids the "annotation tax" problem where every function in the codebase needs markup.
- `unsafe` blocks bypass effect checking (with a warning), because `unsafe` already means "programmer upholds invariants."

**Relationship to existing infrastructure:**

- P10-003 (`@noalloc` equivalent) already exists as a linting rule. The effect system promotes it to a type-checked guarantee.
- P10-001 (no recursion) is a subset of `@nodiverg`.
- `cert-report` already reports P10 violations with structured JSON. Effect violations use the same diagnostic infrastructure.

- [ ] Effect annotation syntax: `@pure`, `@noalloc`, `@noio`, `@nodiverg`.
- [ ] Effect checking pass in the compiler (after type checking, before lowering).
- [ ] Effect annotations on standard library functions (1.0 stdlib).
- [ ] Integration with `cert-report`: effect compliance as a certification artifact.
- [ ] Integration with `fastc context`: effect annotations in API surface dumps.
- [ ] Trait methods can declare effect bounds: `trait Iterator { @noalloc fn next(self: mref(Self)) -> opt(T); }`.

**Definition of Done**

- [ ] `@pure fn add(a: i32, b: i32) -> i32` that calls `printf` produces a compile-time error.
- [ ] Effect violations produce structured diagnostics with the same quality as type errors.
- [ ] `fastc cert-report` includes effect compliance alongside P10 compliance.
- [ ] The standard library has effect annotations on all functions where they apply.

## 2.3 — Async/Await (Optional, Explicit)

> **Requires:** 1.0 (closures for callbacks, traits for a `Future` trait, `Drop` for cancellation cleanup). Benefits from 2.2 (async is an effect — with the effect system, `async fn` can be annotated `@io` and the compiler verifies that non-`@io` code doesn't accidentally suspend).
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

See [docs/competitive-analysis.md](competitive-analysis.md) for detailed positioning against C, Zig, Rust, and V.

FastC's core differentiator is **agent usability** — no other systems language explicitly optimizes for AI coding agents. Combined with source-level C interop, deterministic output, and a path to safety-critical certification, FastC occupies a unique position in the systems programming landscape.
