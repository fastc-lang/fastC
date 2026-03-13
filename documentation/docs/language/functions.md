# Functions

Functions are the basic building blocks of FastC programs.

## Function Declaration

```c
fn function_name(param1: Type1, param2: Type2) -> ReturnType {
    // body
    return value;
}
```

### Example

```c
fn add(a: i32, b: i32) -> i32 {
    return a + b;
}

fn multiply(x: i32, y: i32) -> i32 {
    return x * y;
}
```

## Void Functions

Functions that don't return a value omit the return type:

```c
fn greet(name: slice(u8)) {
    // Do something with name
}

fn log_error(code: i32) {
    // Log the error
}
```

## Calling Functions

```c
let result: i32 = add(10, 20);
greet(c"Alice");
```

## The main Function

Every executable needs a `main` function:

```c
fn main() -> i32 {
    // Program entry point
    return 0;  // Exit code
}
```

The return value becomes the process exit code.

## Parameters

### Pass by Value

By default, parameters are passed by value (copied):

```c
fn double(x: i32) -> i32 {
    return x * 2;
}
```

### Pass by Reference

Use reference types to pass by reference:

```c
fn increment(x: mref(i32)) {
    deref(x) = deref(x) + 1;
}

fn main() -> i32 {
    let value: i32 = 10;
    increment(addr(value));
    // value is now 11
    return 0;
}
```

## Recursion

Functions can call themselves:

```c
fn factorial(n: i32) -> i32 {
    if n <= 1 {
        return 1;
    }
    return n * factorial(n - 1);
}

fn fibonacci(n: i32) -> i32 {
    if n <= 1 {
        return n;
    }
    return fibonacci(n - 1) + fibonacci(n - 2);
}
```

## Forward References

Functions can be called before they're defined:

```c
fn main() -> i32 {
    return helper();  // OK: forward reference
}

fn helper() -> i32 {
    return 42;
}
```

## Unsafe Functions

Functions that require unsafe context to call:

```c
unsafe fn dangerous_operation(ptr: rawm(i32)) {
    deref(ptr) = 42;
}

fn main() -> i32 {
    let x: i32 = 0;
    unsafe {
        dangerous_operation(addr(x));
    }
    return 0;
}
```

See [Unsafe Code](unsafe.md) for details.

## Public Functions

In a library, mark functions as public with `pub`:

```c
pub fn public_api() -> i32 {
    return internal_helper();
}

fn internal_helper() -> i32 {
    return 42;
}
```

## Generated C Code

A FastC function:

```c
fn add(a: i32, b: i32) -> i32 {
    return a + b;
}
```

Compiles to:

```c
int32_t add(int32_t a, int32_t b) {
    int32_t __tmp0;
    if (__builtin_add_overflow(a, b, (&__tmp0))) {
        fc_trap();
    }
    return __tmp0;
}
```

Note the automatic overflow checking for safe arithmetic.

## Best Practices

1. **Keep functions small** - Each function should do one thing
2. **Use descriptive names** - `calculate_total` not `calc`
3. **Document parameters** - Comment what each parameter means
4. **Return early** - Use early returns for error cases
5. **Prefer references for large types** - Avoid copying large structs
