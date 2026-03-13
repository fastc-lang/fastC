# Safety Guarantees

FastC provides memory safety guarantees while maintaining C interoperability.

## Overview

FastC is designed to prevent common C programming errors:

| Error Type | FastC Protection |
|------------|------------------|
| Null pointer dereference | `opt(T)` and `ref(T)` types |
| Buffer overflow | Bounds-checked array access |
| Use after free | Reference types prevent dangling |
| Uninitialized memory | Variables require initialization |
| Type confusion | Strong static typing |

## Safe vs Unsafe Code

### Safe Code

By default, FastC code is safe:

```c
fn safe_function(arr: slice(i32)) -> i32 {
    // Bounds checking at runtime
    return at(arr, 0);
}
```

Safe code cannot:

- Dereference raw pointers
- Call C functions directly
- Perform arbitrary type casts
- Access memory at arbitrary addresses

### Unsafe Code

Unsafe code is explicitly marked:

```c
unsafe fn dangerous() {
    // Can dereference raw pointers
    // Can call extern functions
}

fn mixed() {
    unsafe {
        // Unsafe block within safe function
    }
}
```

## Reference Safety

### Immutable References (`ref(T)`)

```c
fn read_value(r: ref(i32)) -> i32 {
    return deref(r);  // Safe: reference is valid
}
```

Guarantees:

- Reference is non-null
- Points to valid memory
- Cannot be modified through this reference

### Mutable References (`mref(T)`)

```c
fn modify(m: mref(i32)) {
    deref(m) = 42;  // Safe: exclusive access
}
```

Guarantees:

- Reference is non-null
- Points to valid memory
- Exclusive access for mutation

### Raw Pointers (`raw(T)`, `rawm(T)`)

```c
fn use_raw(p: raw(i32)) {
    unsafe {
        // Must be in unsafe block
        let val: i32 = deref(p);
    }
}
```

No guarantees:

- May be null
- May be dangling
- Require unsafe to use

## Optional Type Safety

The `opt(T)` type prevents null pointer errors:

```c
fn get_value() -> opt(i32) {
    return some(42);
    // or: return none(i32);
}

fn use_value() -> i32 {
    let maybe: opt(i32) = get_value();

    // Safe: must handle both cases
    if let value = unwrap_checked(maybe) {
        return value;
    } else {
        return -1;
    }
}
```

## Array Safety

### Bounds Checking

The `at()` function performs bounds checking:

```c
fn access(arr: slice(i32), idx: usize) -> i32 {
    // Runtime bounds check
    return at(arr, idx);
}
```

Out-of-bounds access calls `fc_trap()`.

### Fixed-Size Arrays

```c
let arr: arr(i32, 5) = [1, 2, 3, 4, 5];
let elem: i32 = at(arr, 2);  // Bounds checked
```

## Type Safety

### Strong Typing

All variables have explicit types:

```c
let x: i32 = 42;
let y: f64 = 3.14;
// let z: i32 = y;  // Error: type mismatch
```

### Safe Casts

The `cast()` builtin performs explicit conversions:

```c
let i: i32 = 42;
let u: u32 = cast(u32, i);  // Explicit conversion
```

Unsafe casts require unsafe blocks:

```c
unsafe {
    let ptr: raw(i32) = cast(raw(i32), address);
}
```

## Initialization Safety

All variables must be initialized:

```c
let x: i32 = 0;      // OK: initialized
// let y: i32;       // Error: uninitialized
```

Struct fields must be explicitly set:

```c
struct Point { x: f64, y: f64 }

let p: Point = Point { x: 0.0, y: 0.0 };  // All fields required
```

## Error Handling Safety

### Result Types

```c
enum Error { NotFound, Invalid }

fn parse(input: raw(u8)) -> res(i32, Error) {
    if valid {
        return ok(value);
    }
    return err(Error_Invalid);
}
```

Callers must handle errors:

```c
let result: res(i32, Error) = parse(input);
if is_ok(result) {
    let value: i32 = unwrap(result);
}
```

## Memory Safety Model

### Stack Allocation

Local variables are stack-allocated with automatic cleanup:

```c
fn example() {
    let x: i32 = 42;
    // x is deallocated when function returns
}
```

### Heap Allocation

Heap allocation requires explicit management:

```c
extern "C" {
    unsafe fn malloc(size: usize) -> rawm(u8);
    unsafe fn free(ptr: rawm(u8));
}

fn allocate() {
    unsafe {
        let ptr: rawm(u8) = malloc(100);
        // Must remember to free
        free(ptr);
    }
}
```

## Safety Boundaries

### Extern Functions

All external C functions are unsafe:

```c
extern "C" {
    fn printf(fmt: raw(u8), ...) -> i32;  // Unsafe to call
}

fn print() {
    unsafe {
        discard printf(c"Hello\n");
    }
}
```

### FFI Types

Types passed to C must use `@repr(C)`:

```c
@repr(C)
struct CCompatible {
    x: i32,
    y: i32,
}
```

## What FastC Does NOT Guarantee

In unsafe code:

- **No null checks** - Raw pointers may be null
- **No bounds checks** - Direct pointer arithmetic
- **No lifetime tracking** - Dangling pointers possible
- **No thread safety** - Data races possible

## Best Practices

1. **Minimize unsafe code** - Keep unsafe blocks small
2. **Wrap unsafe in safe APIs** - Hide unsafe details
3. **Use opt(T) for nullable values** - Not raw pointers
4. **Use res(T, E) for errors** - Not sentinel values
5. **Prefer slices over raw pointers** - Bounds checking
6. **Initialize all variables** - No uninitialized memory

## Runtime Traps

When safety checks fail, `fc_trap()` is called:

- Array bounds violation
- Invalid enum value
- Arithmetic overflow (when enabled)

Default behavior: program abort.

## Comparison with C

| C Problem | FastC Solution |
|-----------|----------------|
| `NULL` dereference | `opt(T)` type |
| Buffer overflow | `at()` with bounds check |
| Uninitialized variables | Mandatory initialization |
| Type punning | `cast()` with unsafe |
| Memory leaks | Explicit ownership |
| Undefined behavior | Traps in safe code |

## See Also

- [Unsafe Code](../language/unsafe.md) - Using unsafe blocks
- [Optionals](../language/optionals.md) - Safe null handling
- [Results](../language/results.md) - Safe error handling
- [Runtime](runtime.md) - Trap handler details

