# FastC

FastC is a C‑like systems language designed to be compiled by a **source‑to‑source transpiler** that emits standard C11. The goal is to remove common C footguns while keeping the C toolchain, ABI, and performance model intact.

This repository is the specification and roadmap for the language and transpiler. It is a living design doc, not an implementation yet.

## Goals

- Preserve the C toolchain: clang/gcc, linkers, debuggers, sanitizers, and build systems.
- Make safe code boring and predictable by removing ambiguous syntax and sequencing rules.
- Provide a clear safe/unsafe boundary where remaining UB‑prone operations are explicit.
- Emit stable, readable C that is easy to diff, cache, and audit.
- Keep the front‑end simple and fast to compile.

## Non‑Goals

- Replace C++ or provide a full modern language ecosystem.
- Match Rust‑level whole‑program safety or advanced generic metaprogramming.
- Depend on a custom backend or a bespoke ABI.

## Status

Design phase. The documents in `docs/` describe the current target semantics, lowering model, and the roadmap. Expect changes as we validate feasibility and refine the model.

## Interop Contract

- The transpiler emits standard C11 with no custom ABI.
- Public APIs are defined by generated C headers.
- `@repr(C)` types map directly to C layout rules.
- `bool` lowers to `_Bool`, and `usize/isize` lower to `size_t/ptrdiff_t`.
- `slice(T)` lowers to `{ T* data; size_t len; }`.

## Documents

- `docs/overview.md` — project goals, principles, and scope
- `docs/language.md` — syntax and type system, ownership, borrows, and error handling
- `docs/transpiler.md` — phases and lowering rules into C11
- `docs/safety.md` — safe/unsafe boundary and runtime checks
- `docs/ffi.md` — ABI and interop rules
- `docs/grammar.md` — formal grammar stub and parsing constraints
- `docs/testing.md` — minimal interop test plan
- `docs/checklist.md` — sanity checklist for unambiguous, safe design
- `docs/roadmap.md` — milestones and delivery plan

## Repository Layout

- `README.md`
- `docs/`

## Contributing

Issues and design feedback are welcome. Please keep proposals concrete and tied to a testable behavior in the emitted C.
