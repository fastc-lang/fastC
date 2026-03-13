# C Interoperability

FastC is designed for seamless integration with C code. You can call C functions from FastC and expose FastC functions to C.

## Overview

FastC provides:

- **extern "C"** - Declare external C functions
- **@repr(C)** - C-compatible struct layout
- **Opaque types** - Handle incomplete C types
- **Raw pointers** - Direct memory manipulation

## Type Mappings

| FastC | C |
|-------|---|
| `i8` / `u8` | `int8_t` / `uint8_t` |
| `i16` / `u16` | `int16_t` / `uint16_t` |
| `i32` / `u32` | `int32_t` / `uint32_t` |
| `i64` / `u64` | `int64_t` / `uint64_t` |
| `f32` / `f64` | `float` / `double` |
| `bool` | `_Bool` |
| `usize` / `isize` | `size_t` / `ptrdiff_t` |
| `raw(T)` | `const T*` |
| `rawm(T)` | `T*` |

## Quick Examples

### Calling C

```c
extern "C" {
    fn printf(fmt: raw(u8), ...) -> i32;
}

fn main() -> i32 {
    unsafe {
        discard printf(c"Hello from FastC!\n");
    }
    return 0;
}
```

### Exposing to C

```c
// In FastC
pub fn calculate(x: i32, y: i32) -> i32 {
    return x + y;
}
```

Generated header:

```c
// In generated .h file
int32_t calculate(int32_t x, int32_t y);
```

## Topics

- [Calling C](calling-c.md) - Using C libraries from FastC
- [Exposing APIs](exposing-api.md) - Making FastC functions callable from C
- [Build Systems](build-systems.md) - Integrating with Make, CMake, Meson
