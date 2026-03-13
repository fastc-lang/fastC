# Overview

FastC is a C‑like language that compiles by transpiling to C11. The language is intentionally conservative. It aims to make C safer and more predictable while retaining the existing C toolchain and ABI.

## Design Principles

- **Clarity over cleverness**. The surface syntax is small and unambiguous.
- **Explicit semantics**. No hidden coercions or evaluation order surprises.
- **Safe by default**. Unsafe operations are allowed only in `unsafe` blocks.
- **Toolchain continuity**. Output C should be straightforward and portable.
- **Local reasoning**. Most checks are intra‑function to keep compilation fast.
- **Conservative syntax**. No implicit casts, assignment is statement‑only, and chained binary expressions require parentheses.
- **Explicit effects**. Expression statements are limited to function calls to make side effects obvious.

## Scope

- Language surface: statements, expressions, and a minimal type system that maps cleanly to C.
- Safety: eliminate or guard common C undefined behavior in safe code.
- Transpiler: deterministic lowering rules into C11 with explicit temporaries.

## Out of Scope

- Full macro system or preprocessor at the FastC layer.
- Complex compile‑time evaluation beyond constant folding.
- Whole‑program effect or alias analysis.

## Terminology

- **Safe code**: code that avoids undefined behavior by construction or via inserted runtime checks.
- **Unsafe code**: code that may trigger UB unless the programmer upholds explicit invariants.
- **Lowering**: the deterministic translation from FastC AST to a C AST.
