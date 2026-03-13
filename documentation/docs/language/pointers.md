# Pointers

FastC has four pointer types with different safety guarantees.

## Pointer Types Overview

| Type | Mutable | Safe | Use Case |
|------|---------|------|----------|
| `ref(T)` | No | Yes | Read-only access |
| `mref(T)` | Yes | Yes | Read-write access |
| `raw(T)` | No | No | C interop (const) |
| `rawm(T)` | Yes | No | C interop (mutable) |

## Safe Pointers

### ref(T) - Immutable Reference

Read-only pointer that cannot be null:

```c
fn print_value(r: ref(i32)) {
    let value: i32 = deref(r);
    // Cannot modify through r
}

fn main() -> i32 {
    let x: i32 = 42;
    print_value(addr(x));
    return 0;
}
```

### mref(T) - Mutable Reference

Read-write pointer that cannot be null:

```c
fn increment(r: mref(i32)) {
    deref(r) = deref(r) + 1;
}

fn main() -> i32 {
    let x: i32 = 10;
    increment(addr(x));
    // x is now 11
    return x;
}
```

## addr() - Taking Addresses

Use `addr()` to get a pointer to a variable:

```c
let x: i32 = 42;
let r: ref(i32) = addr(x);
let m: mref(i32) = addr(x);
```

## deref() - Dereferencing

Use `deref()` to access the pointed-to value:

```c
// Reading
let value: i32 = deref(r);

// Writing (mref only)
deref(m) = 100;
```

## Raw Pointers

Raw pointers are for C interoperability and require `unsafe` blocks.

### raw(T) - Raw Immutable Pointer

```c
unsafe fn read_raw(p: raw(i32)) -> i32 {
    return deref(p);
}
```

### rawm(T) - Raw Mutable Pointer

```c
unsafe fn write_raw(p: rawm(i32), value: i32) {
    deref(p) = value;
}
```

### Using Raw Pointers

```c
fn main() -> i32 {
    let x: i32 = 42;
    let p: rawm(i32) = addr(x);

    unsafe {
        deref(p) = 100;
    }

    return x;  // Returns 100
}
```

## Pointer Arithmetic

Raw pointers support arithmetic (in unsafe blocks):

```c
unsafe fn array_access(base: rawm(i32), index: i32) -> i32 {
    // Pointer arithmetic
    let ptr: rawm(i32) = base + index;
    return deref(ptr);
}
```

## Common Patterns

### Out Parameters

```c
fn get_dimensions(width: mref(i32), height: mref(i32)) {
    deref(width) = 800;
    deref(height) = 600;
}

fn main() -> i32 {
    let w: i32 = 0;
    let h: i32 = 0;
    get_dimensions(addr(w), addr(h));
    return w + h;  // 1400
}
```

### Modifying Struct Fields

```c
fn move_point(p: mref(Point), dx: i32, dy: i32) {
    deref(p).x = deref(p).x + dx;
    deref(p).y = deref(p).y + dy;
}
```

### Swapping Values

```c
fn swap(a: mref(i32), b: mref(i32)) {
    let temp: i32 = deref(a);
    deref(a) = deref(b);
    deref(b) = temp;
}
```

### Optional Pointers

For nullable pointers, wrap in `opt`:

```c
fn find_element(data: slice(i32), target: i32) -> opt(ref(i32)) {
    for let i: i32 = 0; i < len(data); i = i + 1 {
        if at(data, i) == target {
            return some(addr(at(data, i)));
        }
    }
    return none(ref(i32));
}
```

## C Interoperability

### Receiving C Pointers

```c
extern "C" {
    fn malloc(size: usize) -> rawm(u8);
    fn free(ptr: rawm(u8));
}

fn allocate_int() -> rawm(i32) {
    unsafe {
        return cast(rawm(i32), malloc(4));
    }
}
```

### Passing Pointers to C

```c
extern "C" {
    fn memset(dest: rawm(u8), value: i32, count: usize) -> rawm(u8);
}

fn zero_memory(buffer: rawm(u8), size: usize) {
    unsafe {
        discard memset(buffer, 0, size);
    }
}
```

## Generated C Code

| FastC | C |
|-------|---|
| `ref(T)` | `const T*` |
| `mref(T)` | `T*` |
| `raw(T)` | `const T*` |
| `rawm(T)` | `T*` |

The difference between ref/raw is semantic - both compile to the same C type, but ref has safety guarantees enforced by FastC.

## Best Practices

1. **Prefer ref/mref** - Use safe pointers when possible
2. **Minimize unsafe** - Keep unsafe blocks small
3. **Document unsafe** - Explain why unsafe is needed
4. **Use opt for nullable** - Wrap raw pointers in opt if they can be null
5. **Avoid pointer arithmetic** - Use slices instead

## See Also

- [Unsafe Code](unsafe.md) - Working with raw pointers
- [Arrays & Slices](arrays-slices.md) - Safe collection access
- [C Interoperability](../c-interop/calling-c.md) - FFI with pointers
