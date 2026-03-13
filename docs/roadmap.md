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

## Future Candidates

- Generics with monomorphization.
- A small standard library focused on slices, strings, and I/O.
- Centralized package registry (building on Git-based deps).
