# Language Guide

FastC is a systems programming language that compiles to C11. It combines familiar C-like syntax with modern safety features.

## Design Philosophy

- **Familiar** - If you know C, you can read FastC
- **Safe by default** - Runtime checks prevent common bugs
- **Opt-in unsafe** - Escape hatches when you need them
- **Zero cost** - Safety checks compile to efficient code

## Language Overview

### Types

FastC has a rich type system:

| Category | Types |
|----------|-------|
| Integers | `i8`, `i16`, `i32`, `i64`, `u8`, `u16`, `u32`, `u64` |
| Floats | `f32`, `f64` |
| Boolean | `bool` |
| Size | `usize`, `isize` |
| Pointers | `ref(T)`, `mref(T)`, `raw(T)`, `rawm(T)` |
| Containers | `arr(T,N)`, `slice(T)` |
| Optional | `opt(T)` |
| Result | `res(T,E)` |
| User-defined | `struct`, `enum` |

### Functions

```c
fn add(a: i32, b: i32) -> i32 {
    return a + b;
}

fn greet() {  // void return
    // ...
}
```

### Control Flow

```c
if condition {
    // ...
} else {
    // ...
}

while condition {
    // ...
}

for let i: i32 = 0; i < 10; i = i + 1 {
    // ...
}

switch value {
    case 1: // ...
    case 2: // ...
    default: // ...
}
```

### Safety Model

FastC code is **safe by default**:

- Array/slice access is bounds-checked
- Integer arithmetic is overflow-checked
- Null/none values must be explicitly handled

Use `unsafe` blocks for operations that bypass safety:

```c
unsafe {
    let ptr: rawm(i32) = get_raw_pointer();
    deref(ptr) = 42;
}
```

## Topics

- [Types](types.md) - Complete type system reference
- [Functions](functions.md) - Function declarations and calling
- [Control Flow](control-flow.md) - Conditionals and loops
- [Structs & Enums](structs-enums.md) - Custom data types
- [Optionals](optionals.md) - Safe nullable values
- [Results](results.md) - Error handling
- [Pointers](pointers.md) - References and raw pointers
- [Arrays & Slices](arrays-slices.md) - Collections
- [Unsafe Code](unsafe.md) - Bypassing safety checks
