# Roadmap

This roadmap is a living plan. Dates are intentionally omitted until implementation starts.

## 0.1 — Rust Harness + Minimal Front End

- Set up a Rust workspace with a single `fastc` CLI crate.
- Implement a lexer (for example, `logos`) with explicit token kinds.
- Implement a hand‑written recursive‑descent parser that enforces the strict grammar.
- Define core AST types and a minimal type checker stub.
- Emit a tiny subset of C11 (functions, `let`, returns).
- Add `insta` snapshot tests for emitted C.
- Add `trycmd` tests for CLI behavior and diagnostics.
- Add a minimal runtime header with `fc_trap` stubs.

**Definition of Done**

- `cargo test` passes with snapshot tests and CLI tests.
- A minimal `.fc` file transpiles to valid C11 and compiles with `clang -std=c11`.
- Deterministic output is verified via snapshots.

## 0.2 — Safety Core + Automation

- Implement `unsafe` blocks and `unsafe fn` checking.
- Add `ref`, `mref`, `raw`, `own`, `slice`, and `arr` types.
- Enforce evaluation order by introducing temporaries in lowering.
- Insert bounds, null, and numeric checks in safe code.
- Generate C headers for exported APIs.
- Add `assert_cmd` + `tempfile` tests that compile emitted C with C11.
- Wire a CI job that runs snapshots and C interop tests on Linux.

**Definition of Done**

- `unsafe` rules are enforced with explicit diagnostics and test coverage.
- Bounds, null, and numeric checks are present in emitted C for safe code.
- C11 compilation tests pass for a representative set of programs.

## 0.3 — Data Types + FFI Hardening

- Add `opt(T)` and `res(T, E)` with explicit lowering.
- Add `enum` with exhaustiveness checking in `switch`.
- Enforce `@repr(C)` for by‑value FFI types.
- Add unaligned access helpers and `memcpy`‑based bitcasts.
- Add the minimal interop matrix from `docs/testing.md`.
- Add ABI layout tests using C `offsetof` and `sizeof`.

**Definition of Done**

- `@repr(C)` validation prevents incompatible FFI layouts.
- The interop test matrix passes with C11 compilers.
- Enum layout and discriminant rules are validated with tests.

## 0.4 — Diagnostics + Deterministic Output

- Add structured diagnostics with spans (`miette` or `codespan‑reporting`).
- Add stable ordering rules for emitted C to guarantee deterministic output.
- Add source maps for error mapping and debugging.
- Add golden tests for error codes and fix‑it hints.

**Definition of Done**

- Diagnostics include spans, codes, and fix‑it suggestions.
- Emitted C is byte‑stable across runs for identical input.
- Source maps round‑trip errors to FastC source lines.

## 0.5 — Tooling and Integration

- Add a `fastc fmt` command to enforce canonical source formatting.
- Add a `fastc check` mode to typecheck without emitting C.
- Add build integration examples (Make, CMake, Meson).
- Add a minimal LSP server or editor integration.

**Definition of Done**

- `fastc fmt` produces stable formatting.
- `fastc check` runs without emitting C and matches compiler semantics.
- Example build integrations compile and run on C11 toolchains.

## Future Candidates

- Generics with monomorphization.
- A small standard library focused on slices, strings, and I/O.
- Build system integration for common C workflows.
