# Transpiler

This document describes the intended compilation pipeline and lowering model.

## Pipeline

1. Parse to an unambiguous AST.
2. Resolve names and types with local inference only.
3. Perform safety checks and insert required runtime checks.
4. Lower to a C‑compatible AST with explicit temporaries.
5. Emit C11 source and headers.

## Evaluation Order

FastC defines left‑to‑right evaluation for:

- Function arguments
- Binary operators
- Assignment evaluation

The transpiler enforces this order by introducing temporaries in the emitted C.

## Lowering Rules

- All control flow is explicit. No implicit short‑circuiting beyond defined operators.
- `defer` lowers to a cleanup path that runs in LIFO order when the scope exits via normal completion, `return`, `break`, or `continue`. Defers do not run on panic/trap.
- Ownership moves lower to assignments that clear the source.
- Bounds and null checks lower to explicit `if` checks that trap on failure.
- Signed overflow, division by zero, and invalid shift counts lower to explicit checks in safe code.
- Calls to `unsafe fn` are only permitted inside `unsafe` blocks.
- `discard(expr)` lowers to evaluation of `expr` with its result ignored.

## Runtime Shim

A small runtime header is expected to provide:

- `fc_alloc(size, align)` and `fc_free(ptr)`
- `fc_trap()`
- `fc_memcpy(dst, src, n)`

The runtime should be minimal and replaceable by users who want custom allocators.

## C Emission

- Emit standard C11 only.
- Avoid macros when possible; prefer `static inline` helpers.
- Keep output deterministic and stable to enable caching and diffing.
- Generate headers for exported functions and types.
