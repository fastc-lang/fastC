# Language

This document defines the intended surface language. It is not a formal grammar, but it is the normative description of the syntax and type system.

## Lexical Rules

- No C preprocessor in FastC source.
- Blocks always use braces.
- Declarations are limited to `let`, `const`, `fn`, `struct`, `enum`, and `opaque`.
- There are no C declarator puzzles or implicit pointer syntax.

## Top‑Level Items

- Function definitions: `fn name(params) -> Type { ... }` or `unsafe fn name(...) -> Type { ... }`
- Type definitions: `struct`, `enum`
- Constants: `const NAME: Type = expr;`
- Opaque types for FFI: `opaque Name;`
- Extern blocks: `extern "C" { ... }` containing function prototypes and type declarations only (no bodies)

## Functions

- Functions always declare an explicit return type, including `-> void`.

## Constants

- `const` initializers must be compile‑time literals or compositions of literals using unary/binary operators (with required parentheses) and `cast`.
- `const` may reference other `const` values.
- `const` does not permit function calls or address‑taking.
- `cstr("...")` and `bytes("...")` are allowed in `const` initializers.

## Opaque Types

- `opaque Name;` declares an incomplete type for FFI.
- Opaque types may only be used behind pointers or in `own(T)`.

## Statements

- `let name: Type = expr;`
- `lhs = expr;` (assignment is a statement, not an expression; the `for` update clause also accepts assignment syntax)
- `if (cond) { ... } else { ... }`
- `while (cond) { ... }`
- `for (init; cond; step) { ... }`
- `switch (expr) { case ... }`
- `break;`, `continue;`, `return expr;`
- `defer { ... }`
- Expression statements are limited to function calls.
- `discard(expr);` evaluates an expression and discards the value explicitly.
- `unsafe { ... }` introduces an unsafe block.

## Expressions

- Arithmetic, comparison, and bitwise operators
- Function calls
- Struct literals
- Field access with `.`
- Explicit `addr(x)` and `deref(p)`
- Indexing via `at(slice_or_array, i)`
- Only one binary operator is allowed per expression level. Combine operators only with parentheses.
- No ternary operator
- No implicit casts
- No assignment expressions

## String Literals

- String literals are only allowed inside `cstr("...")` and `bytes("...")` expressions, and in `extern "C"` declarations.
- `cstr("...")` yields a NUL‑terminated static string (`ref(u8)`).
- `bytes("...")` yields a static byte slice (`slice(u8)`).
- Both forms are compile‑time constants with static storage.

## Operator Type Rules

- Binary operators require operands of the same type after explicit casts.
- Comparisons are defined only between equal types.
- There are no implicit numeric promotions.
- Binary operators evaluate left operand before right operand.
- `&&` and `||` short‑circuit left‑to‑right.

## Condition Expressions

- Conditions in `if`, `while`, and `for` must be `bool`.
- There is no implicit integer‑to‑bool conversion.

## For Loops

- `init` and `step` are limited to `let`, assignment, or function call forms.

## Initialization

- All `let` declarations require an initializer.

## Switch

- `switch` is permitted only on integer and enum types.
- `case` labels must be `const` expressions of the same type as the `switch` expression.

## Array Sizes

- `arr(T, N)` requires `N` to be a `const` expression of type `usize`.

## Types

### Primitive Types

- Signed integers: `i8`, `i16`, `i32`, `i64`
- Unsigned integers: `u8`, `u16`, `u32`, `u64`
- Floating point: `f32`, `f64`
- Boolean: `bool`
- Pointer-sized: `usize`, `isize`
- `void` (return type only)

### References and Pointers

- `ref(T)`: non‑null, immutable reference
- `mref(T)`: non‑null, mutable reference with exclusive access
- `raw(T)`: raw pointer, nullable, unsafe to dereference
- `rawm(T)`: mutable raw pointer, nullable, unsafe to dereference

### Ownership

- `own(T)`: owning pointer to `T`, move‑only

### Aggregates

- `arr(T, N)`: fixed‑size array
- `slice(T)`: view over contiguous elements with length
- `struct` and `enum` types

### Function Types

- Function types are written as `fn(...) -> T` or `unsafe fn(...) -> T`.
- The return type may be `void`.

### Enums

- Enums are distinct types; conversions to and from integers require `cast`.

### Option and Result

- `opt(T)`: optional value, used for nullable semantics in safe code
- `res(T, E)`: result type for error handling
- `opt` and `res` are not permitted in `extern` function signatures.

## Conversions

- Explicit casts use `cast(T, expr)`.
- There are no implicit numeric widenings or narrowings.
- Conversions between `ref/mref` and `raw/rawm` are explicit via helper functions (for example, `to_raw`).

## Attributes

- `@repr(C)` is allowed on `struct` and `enum` to force C layout rules.
- Enums default to `@repr(i32)` unless explicitly annotated.
- Valid enum reprs: `@repr(i8)`, `@repr(u8)`, `@repr(i16)`, `@repr(u16)`, `@repr(i32)`, `@repr(u32)`, `@repr(i64)`, `@repr(u64)`.

## Ownership and Borrowing

- `own(T)` values are move‑only.
- `ref(T)` borrows immutably and can be aliased.
- `mref(T)` borrows mutably and is exclusive.
- Borrows are local and lexical, checked per function.

## Nullability

- `ref` and `mref` are non‑null by construction.
- `raw` and `rawm` are nullable and require `unsafe` to dereference.
- `opt(T)` provides a safe way to represent absence without raw pointers.

## Error Handling

- `res(T, E)` is the primary error type.
- No exceptions or unwinding.
- `panic` is implemented as an abort trap in the runtime.

## Unsafe

- Any dereference of `raw/rawm`, pointer arithmetic, or FFI call requires an `unsafe` block.
- Declaring `unsafe fn` marks a function as unsafe to call.
- Calls to an `unsafe fn` require an `unsafe` block, even if the function body is safe.

## Numeric Semantics

- Signed overflow in safe code traps.
- Unsigned overflow wraps.
- Division by zero traps.
- Shift counts outside the type width trap.
