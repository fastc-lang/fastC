# Unsafe Code

FastC is safe by default, but provides `unsafe` for low-level operations.

## What Unsafe Enables

Inside an `unsafe` block, you can:

- Dereference raw pointers (`raw(T)`, `rawm(T)`)
- Call unsafe functions
- Bypass runtime safety checks

## Unsafe Blocks

```c
fn example() {
    let x: i32 = 42;
    let ptr: rawm(i32) = addr(x);

    unsafe {
        // Can dereference raw pointer here
        deref(ptr) = 100;
    }

    // Outside unsafe: safe code only
}
```

## Unsafe Functions

Declare functions that require unsafe context:

```c
unsafe fn dangerous_read(ptr: raw(i32)) -> i32 {
    return deref(ptr);
}

unsafe fn dangerous_write(ptr: rawm(i32), value: i32) {
    deref(ptr) = value;
}
```

### Calling Unsafe Functions

```c
fn main() -> i32 {
    let x: i32 = 42;

    unsafe {
        let value: i32 = dangerous_read(addr(x));
        dangerous_write(addr(x), value + 1);
    }

    return x;
}
```

## Safe Wrappers

Wrap unsafe operations in safe APIs:

```c
// Unsafe low-level function
unsafe fn raw_get(ptr: raw(i32), index: i32) -> i32 {
    return deref(ptr + index);
}

// Safe wrapper with bounds checking
fn safe_get(data: slice(i32), index: i32) -> opt(i32) {
    if index < 0 || index >= len(data) {
        return none(i32);
    }
    unsafe {
        return some(raw_get(data.data, index));
    }
}
```

## Common Use Cases

### Memory Allocation

```c
extern "C" {
    fn malloc(size: usize) -> rawm(u8);
    fn free(ptr: rawm(u8));
}

fn allocate(size: usize) -> rawm(u8) {
    unsafe {
        return malloc(size);
    }
}

fn deallocate(ptr: rawm(u8)) {
    unsafe {
        free(ptr);
    }
}
```

### Type Punning

```c
unsafe fn reinterpret_as_bytes(value: raw(i32)) -> raw(u8) {
    return cast(raw(u8), value);
}

fn get_bytes(x: i32) -> arr(u8, 4) {
    let bytes: arr(u8, 4) = [0, 0, 0, 0];
    unsafe {
        let src: raw(u8) = reinterpret_as_bytes(addr(x));
        for let i: i32 = 0; i < 4; i = i + 1 {
            at(bytes, i) = deref(src + i);
        }
    }
    return bytes;
}
```

### FFI Calls

```c
extern "C" {
    fn strlen(s: raw(u8)) -> usize;
    fn memcpy(dest: rawm(u8), src: raw(u8), n: usize) -> rawm(u8);
}

fn string_length(s: slice(u8)) -> usize {
    unsafe {
        return strlen(s.data);
    }
}

fn copy_memory(dest: rawm(u8), src: raw(u8), len: usize) {
    unsafe {
        discard memcpy(dest, src, len);
    }
}
```

### Hardware Access

```c
unsafe fn read_port(port: u16) -> u8 {
    // Platform-specific I/O
    let ptr: raw(u8) = cast(raw(u8), port);
    return deref(ptr);
}

unsafe fn write_port(port: u16, value: u8) {
    let ptr: rawm(u8) = cast(rawm(u8), port);
    deref(ptr) = value;
}
```

## What Remains Checked

Even in unsafe blocks:

- Type checking is still enforced
- Syntax errors are still caught
- Function signatures must match

Unsafe only bypasses runtime safety checks.

## Unsafe Guidelines

### Keep Unsafe Blocks Small

```c
// Bad: large unsafe block
unsafe {
    // 50 lines of code...
}

// Good: minimal unsafe
let result: i32 = unsafe {
    deref(ptr)
};
// Rest of function is safe
```

### Document Unsafe Invariants

```c
// SAFETY: ptr must be non-null and point to valid i32
// The caller guarantees the pointer is valid for the lifetime of this call
unsafe fn read_value(ptr: raw(i32)) -> i32 {
    return deref(ptr);
}
```

### Encapsulate Unsafe

```c
// Internal unsafe implementation
unsafe fn raw_swap(a: rawm(i32), b: rawm(i32)) {
    let temp: i32 = deref(a);
    deref(a) = deref(b);
    deref(b) = temp;
}

// Safe public API
fn swap(a: mref(i32), b: mref(i32)) {
    unsafe {
        raw_swap(a, b);
    }
}
```

## Unsafe vs Safe

| Operation | Safe | Unsafe |
|-----------|------|--------|
| `ref(T)` dereference | Yes | Yes |
| `mref(T)` dereference | Yes | Yes |
| `raw(T)` dereference | No | Yes |
| `rawm(T)` dereference | No | Yes |
| Pointer arithmetic | No | Yes |
| Call unsafe fn | No | Yes |
| Bounds-checked access | Yes | Optional |
| Overflow checking | Yes | Optional |

## Disabling Runtime Checks

In release builds with `--release`, some checks may be optimized away. For explicit unsafe arithmetic:

```c
unsafe {
    // No overflow check in unsafe
    let result: i32 = a + b;
}
```

## Best Practices

1. **Minimize unsafe surface area** - Keep unsafe blocks as small as possible
2. **Create safe abstractions** - Wrap unsafe in safe APIs
3. **Document safety requirements** - Explain what callers must guarantee
4. **Review unsafe carefully** - Unsafe code needs extra scrutiny
5. **Test unsafe code thoroughly** - Bugs in unsafe are harder to catch

## See Also

- [Pointers](pointers.md) - Pointer types and operations
- [C Interoperability](../c-interop/calling-c.md) - FFI with C
- [Safety Guarantees](../reference/safety.md) - What safe code prevents
