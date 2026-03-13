# Sanity Checklist

Use this checklist when updating any FastC document. The goal is to keep the language explicit, unambiguous, and interoperable with C.

## Syntax and Parsing

- Every construct is grammar‑representable with no ambiguous parse.
- Any new syntax avoids C declarator patterns.
- Expression rules remain “one binary operator per expression level” unless parenthesized.
- Expression statements remain call‑only or `discard(...)`.
- Assignment remains statement‑only.
- Conditions remain `bool` only with no implicit conversion.

## Semantics and Safety

- All side effects are explicit and obvious at the statement level.
- Evaluation order is left‑to‑right and enforced in lowering.
- Unsafe operations require `unsafe { ... }` or `unsafe fn` and explicit call‑site checks.
- No implicit casts, coercions, or numeric promotions.
- Numeric traps are defined for signed overflow, division by zero, and invalid shift counts.

## Types and Layout

- `void` remains return‑only.
- `@repr(C)` is required for by‑value FFI types.
- Enum representation is explicit (`@repr(i32)` default) and consistent.
- `slice(T)` layout is `{ T* data; size_t len; }`.
- `opt/res` remain disallowed in extern signatures unless layout is defined.

## Interop and C Output

- Emitted C stays within standard C11.
- Exported symbols are unmangled and match generated headers.
- No hidden ABI changes or toolchain‑specific extensions.

## Const and Compile‑Time Rules

- `const` expressions are a strict subset with explicit parentheses.
- Array sizes are `const` expressions of type `usize`.
- `case` labels are `const` expressions and match the `switch` type.

