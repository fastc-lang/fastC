# Testing

This document defines the minimal tests required to validate that FastC emits working and testable C code. These tests are intentionally small and designed to run with a plain C11 toolchain.

## Test Philosophy

- Focus on ABI and layout correctness first.
- Keep tests deterministic and small.
- The “golden” reference is standard C11 behavior.

## Minimal Interop Test Suite

Each test should compile and link using a standard C11 compiler (for example, `clang -std=c11` or `gcc -std=c11`).

### 1) Struct Layout

**Goal**: `@repr(C)` layout must match C.

- Define a `@repr(C)` FastC struct with mixed fields.
- Emit a header and a C file.
- In a C test, use `offsetof` and `sizeof` to compare expected offsets.

### 2) Enum Representation

**Goal**: Enum size and values match the declared underlying type.

- Define an enum with explicit discriminants.
- In C, verify `sizeof(enum)` and value equality.

### 3) Bool ABI

**Goal**: `bool` maps to `_Bool`.

- Expose a FastC function returning `bool`.
- In C, assert `sizeof(_Bool)` equals the size of the returned type.

### 4) Pointer‑Sized Integers

**Goal**: `usize/isize` map to `size_t/ptrdiff_t`.

- Expose functions returning `usize` and `isize`.
- In C, verify `sizeof(size_t)` and `sizeof(ptrdiff_t)`.

### 5) Slice ABI

**Goal**: `slice(T)` lowers to `{ T* data; size_t len; }`.

- Expose a FastC function that accepts a `slice(u8)`.
- In C, pass a struct `{ uint8_t* data; size_t len; }` and verify behavior.

### 6) Calling Convention

**Goal**: Extern calls are compatible with C calling convention.

- Declare a C function and call it from FastC via `extern "C"`.
- Confirm correct argument passing and return value.

### 7) Name Stability

**Goal**: Exported names match generated headers.

- Export a FastC function.
- In C, include the generated header and link without symbol lookup hacks.

## Suggested Harness

A minimal harness can be implemented later, but the design assumptions are:

- A `tests/interop/` directory with C test files.
- A build step that compiles FastC to C, then compiles C tests.
- A single driver script that returns non‑zero on any failure.

