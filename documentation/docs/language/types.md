# Types

FastC has a strong, static type system. All variables must have a known type at compile time.

## Primitive Types

### Integers

| Type | Size | Range |
|------|------|-------|
| `i8` | 8-bit | -128 to 127 |
| `i16` | 16-bit | -32,768 to 32,767 |
| `i32` | 32-bit | -2^31 to 2^31-1 |
| `i64` | 64-bit | -2^63 to 2^63-1 |
| `u8` | 8-bit | 0 to 255 |
| `u16` | 16-bit | 0 to 65,535 |
| `u32` | 32-bit | 0 to 2^32-1 |
| `u64` | 64-bit | 0 to 2^64-1 |

```c
let a: i32 = 42;
let b: u8 = 255;
let c: i64 = 9223372036854775807;
```

### Size Types

| Type | Description |
|------|-------------|
| `usize` | Unsigned pointer-sized integer |
| `isize` | Signed pointer-sized integer |

These map to `size_t` and `ptrdiff_t` in C.

```c
let len: usize = 100;
let offset: isize = -10;
```

### Floating Point

| Type | Size | Precision |
|------|------|-----------|
| `f32` | 32-bit | ~7 decimal digits |
| `f64` | 64-bit | ~15 decimal digits |

```c
let pi: f32 = 3.14159;
let e: f64 = 2.718281828459045;
```

### Boolean

```c
let flag: bool = true;
let done: bool = false;
```

## Literals

### Integer Literals

```c
let decimal: i32 = 42;
let hex: i32 = 0xFF;
let binary: i32 = 0b1010;
let with_underscores: i64 = 1_000_000;
```

### Float Literals

```c
let f: f64 = 3.14;
let scientific: f64 = 1.5e10;
```

### String Literals

```c
let s: slice(u8) = c"Hello, world!";
```

The `c"..."` syntax creates a C-compatible string (null-terminated).

## Type Annotations

Variables require explicit type annotations:

```c
let x: i32 = 10;        // Required
let y = 10;             // Error: missing type
```

## Type Casting

Use `cast()` to convert between types:

```c
let a: i32 = 42;
let b: i64 = cast(i64, a);      // Widening (safe)
let c: i16 = cast(i16, a);      // Narrowing (may truncate)
let d: f64 = cast(f64, a);      // Int to float
```

## Compound Types

### Arrays

Fixed-size arrays with compile-time known length:

```c
let arr: arr(i32, 5) = [1, 2, 3, 4, 5];
```

See [Arrays & Slices](arrays-slices.md) for details.

### Slices

Dynamic views into contiguous memory:

```c
let data: slice(i32) = get_data();
```

See [Arrays & Slices](arrays-slices.md) for details.

### Optionals

Values that may or may not be present:

```c
let maybe: opt(i32) = some(42);
let nothing: opt(i32) = none(i32);
```

See [Optionals](optionals.md) for details.

### Results

Values that may be a success or an error:

```c
let result: res(i32, Error) = ok(42);
let failure: res(i32, Error) = err(Error::NotFound);
```

See [Results](results.md) for details.

### Pointers

References and raw pointers for memory access:

```c
let r: ref(i32) = addr(x);      // Immutable reference
let m: mref(i32) = addr(x);     // Mutable reference
let p: raw(i32) = addr(x);      // Raw immutable pointer
let q: rawm(i32) = addr(x);     // Raw mutable pointer
```

See [Pointers](pointers.md) for details.

### Structs

User-defined composite types:

```c
struct Point {
    x: i32,
    y: i32,
}
```

See [Structs & Enums](structs-enums.md) for details.

### Enums

User-defined sum types:

```c
enum Color {
    Red,
    Green,
    Blue,
}
```

See [Structs & Enums](structs-enums.md) for details.

## Type Mappings to C

| FastC | C11 |
|-------|-----|
| `i8` | `int8_t` |
| `i16` | `int16_t` |
| `i32` | `int32_t` |
| `i64` | `int64_t` |
| `u8` | `uint8_t` |
| `u16` | `uint16_t` |
| `u32` | `uint32_t` |
| `u64` | `uint64_t` |
| `f32` | `float` |
| `f64` | `double` |
| `bool` | `_Bool` |
| `usize` | `size_t` |
| `isize` | `ptrdiff_t` |
| `ref(T)` | `const T*` |
| `mref(T)` | `T*` |
| `raw(T)` | `const T*` |
| `rawm(T)` | `T*` |
