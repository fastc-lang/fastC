# Advanced Examples

Real-world patterns and techniques for FastC programs.

## FFI with libc

Calling C library functions from FastC:

```c
// Declare libc functions
extern "C" {
    unsafe fn malloc(size: usize) -> rawm(u8);
    unsafe fn free(ptr: rawm(u8)) -> void;
    unsafe fn memset(ptr: rawm(u8), value: i32, size: usize) -> rawm(u8);
    unsafe fn strlen(s: raw(u8)) -> usize;
    unsafe fn puts(s: raw(u8)) -> i32;
}

// Safe wrapper for allocation
unsafe fn alloc_zeroed(size: usize) -> rawm(u8) {
    let ptr: rawm(u8) = malloc(size);
    let result: rawm(u8) = memset(ptr, 0, size);
    discard(result);
    return ptr;
}

fn main() -> i32 {
    unsafe {
        let buffer: rawm(u8) = alloc_zeroed(cast(usize, 100));
        // Use buffer...
        free(buffer);
    }
    return 0;
}
```

**Pattern:** Wrap unsafe FFI calls in safe functions with proper error handling.

## State Machine

Using enums for state machines:

```c
enum LightState {
    Red,
    Yellow,
    Green,
}

enum LightEvent {
    Timer,
    Emergency,
    Reset,
}

fn transition(current: LightState, event: LightEvent) -> LightState {
    switch (event) {
        case LightEvent_Emergency: {
            return LightState_Red;
        }
        case LightEvent_Reset: {
            return LightState_Red;
        }
        case LightEvent_Timer: {
            switch (current) {
                case LightState_Red: { return LightState_Green; }
                case LightState_Green: { return LightState_Yellow; }
                case LightState_Yellow: { return LightState_Red; }
            }
        }
    }
}

fn get_duration(state: LightState) -> i32 {
    switch (state) {
        case LightState_Red: { return 30; }
        case LightState_Yellow: { return 5; }
        case LightState_Green: { return 25; }
    }
}
```

**Pattern:** Use nested switch statements for state/event combinations.

## Error Handling Patterns

Multiple error handling approaches:

### Pattern 1: Optional Return

```c
fn find_char(s: slice(u8), target: u8, length: i32) -> opt(i32) {
    let i: i32 = 0;
    while (i < length) {
        if (at(s, cast(usize, i)) == target) {
            return some(i);
        }
        i = (i + 1);
    }
    return none(i32);
}
```

### Pattern 2: Error Code via Out Parameter

```c
fn divide_safe(a: i32, b: i32, result: mref(i32)) -> i32 {
    if (b == 0) {
        return -1;  // Error
    }
    deref(result) = (a / b);
    return 0;  // Success
}
```

### Pattern 3: Chained Operations

```c
fn chain_operations(input: i32) -> opt(i32) {
    let step1: i32 = process_input(input);
    if (step1 < 0) {
        return none(i32);
    }

    let step2: i32 = process_input(step1);
    if (step2 < 0) {
        return none(i32);
    }

    return some(step2);
}
```

## Algorithms

Common algorithm implementations:

### Binary Search

```c
fn binary_search(s: slice(i32), target: i32, low: i32, high: i32) -> i32 {
    if (low > high) {
        return -1;
    }

    let mid: i32 = (low + ((high - low) / 2));
    let mid_val: i32 = at(s, cast(usize, mid));

    if (mid_val == target) {
        return mid;
    } else if (mid_val > target) {
        return binary_search(s, target, low, (mid - 1));
    } else {
        return binary_search(s, target, (mid + 1), high);
    }
}
```

### GCD (Euclidean Algorithm)

```c
fn gcd(a: i32, b: i32) -> i32 {
    while (b != 0) {
        let temp: i32 = b;
        b = (a % b);
        a = temp;
    }
    return a;
}

fn lcm(a: i32, b: i32) -> i32 {
    return ((a / gcd(a, b)) * b);
}
```

### Fast Power (Exponentiation by Squaring)

```c
fn pow(base: i32, exp: i32) -> i32 {
    if (exp == 0) { return 1; }
    if (exp == 1) { return base; }

    let result: i32 = 1;
    let b: i32 = base;
    let e: i32 = exp;

    while (e > 0) {
        if ((e % 2) == 1) {
            result = (result * b);
        }
        b = (b * b);
        e = (e / 2);
    }
    return result;
}
```

## Recursion Patterns

### Factorial

```c
fn factorial(n: i32) -> i32 {
    if (n <= 1) {
        return 1;
    }
    return (n * factorial(n - 1));
}
```

### Fibonacci

```c
fn fibonacci(n: i32) -> i32 {
    if (n <= 0) { return 0; }
    if (n == 1) { return 1; }
    return (fibonacci(n - 1) + fibonacci(n - 2));
}
```

### Tail-Recursive Sum

```c
fn sum_to_n(n: i32, acc: i32) -> i32 {
    if (n <= 0) {
        return acc;
    }
    return sum_to_n((n - 1), (acc + n));
}
```

## Bitwise Operations

Working with flags and bit manipulation:

```c
const FLAG_READ: u32 = 1;      // 0001
const FLAG_WRITE: u32 = 2;     // 0010
const FLAG_EXEC: u32 = 4;      // 0100

fn has_flag(flags: u32, flag: u32) -> bool {
    return ((flags & flag) != 0);
}

fn set_flag(flags: u32, flag: u32) -> u32 {
    return (flags | flag);
}

fn clear_flag(flags: u32, flag: u32) -> u32 {
    return (flags & (~flag));
}

fn toggle_flag(flags: u32, flag: u32) -> u32 {
    return (flags ^ flag);
}
```

## Type Patterns

### Opaque Handle Pattern

```c
@repr(C)
struct Context {
    data: rawm(u8),
    size: usize,
}

pub fn context_create() -> rawm(Context) {
    unsafe {
        let ctx: rawm(Context) = cast(rawm(Context), malloc(sizeof_context()));
        return ctx;
    }
}

pub fn context_destroy(ctx: rawm(Context)) {
    unsafe {
        free(cast(rawm(u8), ctx));
    }
}

pub fn context_process(ctx: rawm(Context), input: raw(u8)) -> i32 {
    // Use context...
    return 0;
}
```

### Builder Pattern

```c
struct Config {
    timeout: i32,
    retries: i32,
    debug: bool,
}

fn config_new() -> Config {
    return Config { timeout: 30, retries: 3, debug: false };
}

fn config_with_timeout(cfg: Config, timeout: i32) -> Config {
    return Config { timeout: timeout, retries: cfg.retries, debug: cfg.debug };
}

fn config_with_debug(cfg: Config, debug: bool) -> Config {
    return Config { timeout: cfg.timeout, retries: cfg.retries, debug: debug };
}
```

## Unsafe Patterns

Safe wrappers for unsafe operations:

```c
extern "C" {
    unsafe fn malloc(size: usize) -> rawm(u8);
    unsafe fn free(ptr: rawm(u8));
}

// Safe allocation returning optional
fn safe_alloc(size: usize) -> opt(rawm(u8)) {
    unsafe {
        let ptr: rawm(u8) = malloc(size);
        if (ptr == cast(rawm(u8), 0)) {
            return none(rawm(u8));
        }
        return some(ptr);
    }
}

// Safe deallocation
fn safe_free(ptr: rawm(u8)) {
    unsafe {
        free(ptr);
    }
}
```

## Best Practices

1. **Wrap unsafe code** - Expose safe APIs that hide unsafe internals
2. **Use enums for states** - Makes invalid states unrepresentable
3. **Return optionals** - Instead of sentinel values like -1 or NULL
4. **Document ownership** - Who allocates, who frees?
5. **Check all errors** - Don't ignore return codes
6. **Keep functions small** - Single responsibility principle

## See Also

- [Tutorials](tutorials.md) - Basic concepts
- [Unsafe Code](../language/unsafe.md) - Unsafe patterns
- [C Interop](../c-interop/index.md) - FFI details

