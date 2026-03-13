# Calling C from FastC

FastC can call any C function through the `extern "C"` mechanism.

## External Declarations

Declare C functions inside an `extern "C"` block:

```c
extern "C" {
    fn malloc(size: usize) -> rawm(u8);
    fn free(ptr: rawm(u8));
    fn printf(fmt: raw(u8), ...) -> i32;
}
```

## Calling External Functions

External functions require `unsafe`:

```c
fn main() -> i32 {
    unsafe {
        let ptr: rawm(u8) = malloc(1024);
        // Use ptr...
        free(ptr);
    }
    return 0;
}
```

## Type Conversions

### Pointers

C pointers map to FastC raw pointers:

```c
// C: void* malloc(size_t size);
fn malloc(size: usize) -> rawm(u8);

// C: int strcmp(const char* s1, const char* s2);
fn strcmp(s1: raw(u8), s2: raw(u8)) -> i32;
```

### Structs

Use `@repr(C)` for C-compatible layout:

```c
@repr(C)
struct Point {
    x: f64,
    y: f64,
}

extern "C" {
    fn process_point(p: raw(Point));
}
```

### Strings

C strings are `raw(u8)`:

```c
extern "C" {
    fn strlen(s: raw(u8)) -> usize;
    fn strcpy(dest: rawm(u8), src: raw(u8)) -> rawm(u8);
}

fn get_length(s: slice(u8)) -> usize {
    unsafe {
        return strlen(s.data);
    }
}
```

## Opaque Types

For C types you don't need to know the layout:

```c
opaque FILE;

extern "C" {
    fn fopen(path: raw(u8), mode: raw(u8)) -> rawm(FILE);
    fn fclose(file: rawm(FILE)) -> i32;
    fn fread(ptr: rawm(u8), size: usize, count: usize, file: rawm(FILE)) -> usize;
}
```

## Variadic Functions

Use `...` for variadic functions:

```c
extern "C" {
    fn printf(fmt: raw(u8), ...) -> i32;
    fn sprintf(buf: rawm(u8), fmt: raw(u8), ...) -> i32;
}
```

## Common libc Functions

### Memory

```c
extern "C" {
    fn malloc(size: usize) -> rawm(u8);
    fn calloc(count: usize, size: usize) -> rawm(u8);
    fn realloc(ptr: rawm(u8), size: usize) -> rawm(u8);
    fn free(ptr: rawm(u8));
    fn memcpy(dest: rawm(u8), src: raw(u8), n: usize) -> rawm(u8);
    fn memset(s: rawm(u8), c: i32, n: usize) -> rawm(u8);
    fn memmove(dest: rawm(u8), src: raw(u8), n: usize) -> rawm(u8);
}
```

### Strings

```c
extern "C" {
    fn strlen(s: raw(u8)) -> usize;
    fn strcpy(dest: rawm(u8), src: raw(u8)) -> rawm(u8);
    fn strncpy(dest: rawm(u8), src: raw(u8), n: usize) -> rawm(u8);
    fn strcmp(s1: raw(u8), s2: raw(u8)) -> i32;
    fn strcat(dest: rawm(u8), src: raw(u8)) -> rawm(u8);
}
```

### I/O

```c
extern "C" {
    fn puts(s: raw(u8)) -> i32;
    fn putchar(c: i32) -> i32;
    fn getchar() -> i32;
}
```

### Math

```c
extern "C" {
    fn sin(x: f64) -> f64;
    fn cos(x: f64) -> f64;
    fn sqrt(x: f64) -> f64;
    fn pow(base: f64, exp: f64) -> f64;
    fn abs(x: i32) -> i32;
}
```

## Safe Wrappers

Create safe APIs around unsafe C functions:

```c
extern "C" {
    fn malloc(size: usize) -> rawm(u8);
    fn free(ptr: rawm(u8));
}

fn allocate(size: usize) -> opt(rawm(u8)) {
    unsafe {
        let ptr: rawm(u8) = malloc(size);
        if ptr == cast(rawm(u8), 0) {
            return none(rawm(u8));
        }
        return some(ptr);
    }
}

fn deallocate(ptr: rawm(u8)) {
    unsafe {
        free(ptr);
    }
}
```

## Error Handling

### errno-style

```c
extern "C" {
    fn errno() -> i32;
    fn strerror(errnum: i32) -> raw(u8);
}

fn checked_operation() -> res(i32, i32) {
    unsafe {
        let result: i32 = some_c_function();
        if result < 0 {
            return err(errno());
        }
        return ok(result);
    }
}
```

## Callbacks

For C functions that take callbacks:

```c
// Callback type in C: int (*compare)(const void*, const void*)
// In FastC, use a function pointer type

extern "C" {
    fn qsort(
        base: rawm(u8),
        nmemb: usize,
        size: usize,
        compar: raw(fn(raw(u8), raw(u8)) -> i32)
    );
}
```

## Example: Using zlib

```c
opaque z_stream;

extern "C" {
    fn compress(
        dest: rawm(u8),
        destLen: mref(usize),
        source: raw(u8),
        sourceLen: usize
    ) -> i32;

    fn uncompress(
        dest: rawm(u8),
        destLen: mref(usize),
        source: raw(u8),
        sourceLen: usize
    ) -> i32;
}

const Z_OK: i32 = 0;

fn compress_data(input: slice(u8), output: slice(u8)) -> res(usize, i32) {
    let out_len: usize = len(output);
    unsafe {
        let result: i32 = compress(
            output.data,
            addr(out_len),
            input.data,
            len(input)
        );
        if result != Z_OK {
            return err(result);
        }
    }
    return ok(out_len);
}
```

## Best Practices

1. **Wrap unsafe in safe APIs** - Hide unsafe details from users
2. **Check return values** - C functions often signal errors via return values
3. **Handle NULL** - C functions may return NULL; convert to opt(T)
4. **Use @repr(C)** - Ensure struct layout matches C
5. **Document ownership** - Note who owns/frees memory

## See Also

- [Unsafe Code](../language/unsafe.md) - Unsafe blocks and functions
- [Pointers](../language/pointers.md) - Raw pointer types
- [Exposing APIs](exposing-api.md) - Making FastC callable from C
