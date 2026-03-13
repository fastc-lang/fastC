# Exposing FastC APIs to C

FastC functions can be called from C code, making it easy to integrate into existing projects.

## Public Functions

Mark functions as public with `pub`:

```c
pub fn calculate(x: i32, y: i32) -> i32 {
    return x + y;
}

// Private helper (not in header)
fn internal_helper() -> i32 {
    return 42;
}
```

## Generating Headers

Use `--emit-header` to generate a C header:

```bash
fastc compile src/lib.fc -o build/lib.c --emit-header
```

This creates `build/lib.h`:

```c
#ifndef LIB_H
#define LIB_H

#include <stdint.h>
#include <stdbool.h>

int32_t calculate(int32_t x, int32_t y);

#endif /* LIB_H */
```

## Struct Compatibility

Use `@repr(C)` for C-compatible struct layout:

```c
@repr(C)
pub struct Point {
    x: f64,
    y: f64,
}

pub fn create_point(x: f64, y: f64) -> Point {
    return Point { x: x, y: y };
}

pub fn point_distance(p: raw(Point)) -> f64 {
    let dx: f64 = deref(p).x;
    let dy: f64 = deref(p).y;
    return sqrt(dx * dx + dy * dy);
}
```

Generated header:

```c
typedef struct Point {
    double x;
    double y;
} Point;

Point create_point(double x, double y);
double point_distance(const Point* p);
```

## Enum Compatibility

Simple enums map directly to C enums:

```c
pub enum Status {
    Pending,
    Running,
    Complete,
    Failed,
}

pub fn get_status() -> Status {
    return Status::Running;
}
```

Generated:

```c
typedef enum Status {
    Status_Pending,
    Status_Running,
    Status_Complete,
    Status_Failed,
} Status;

Status get_status(void);
```

## Returning Optional Values

For functions that may fail, use sentinel values for C compatibility:

```c
// Return -1 for "not found" (C idiom)
pub fn find_index(data: slice(i32), target: i32) -> i32 {
    for let i: i32 = 0; i < len(data); i = i + 1 {
        if at(data, i) == target {
            return i;
        }
    }
    return -1;  // Not found
}
```

Or use out parameters:

```c
// Return success/failure, value via out parameter
pub fn try_parse(input: raw(u8), result: mref(i32)) -> bool {
    // Parse input...
    if valid {
        deref(result) = value;
        return true;
    }
    return false;
}
```

## Error Handling for C

Use integer error codes:

```c
pub const ERROR_NONE: i32 = 0;
pub const ERROR_INVALID: i32 = -1;
pub const ERROR_NOT_FOUND: i32 = -2;

pub fn process(input: raw(u8), output: mref(i32)) -> i32 {
    if input == null {
        return ERROR_INVALID;
    }
    // Process...
    deref(output) = result;
    return ERROR_NONE;
}
```

## Memory Management

### Allocation Functions

```c
pub fn create_buffer(size: usize) -> rawm(u8) {
    unsafe {
        return malloc(size);
    }
}

pub fn destroy_buffer(buf: rawm(u8)) {
    unsafe {
        free(buf);
    }
}
```

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
        // Initialize...
        return ctx;
    }
}

pub fn context_destroy(ctx: rawm(Context)) {
    unsafe {
        // Cleanup...
        free(cast(rawm(u8), ctx));
    }
}

pub fn context_process(ctx: rawm(Context), input: raw(u8)) -> i32 {
    // Use context...
    return 0;
}
```

## Callback Support

Accept function pointers from C:

```c
pub fn set_callback(cb: raw(fn(i32) -> i32)) {
    // Store callback...
}

pub fn invoke_callback(value: i32) -> i32 {
    unsafe {
        return deref(callback)(value);
    }
}
```

## Using from C

### Compiling Together

```bash
# Generate C from FastC
fastc compile src/mylib.fc -o build/mylib.c --emit-header

# Compile everything
gcc -c build/mylib.c -o build/mylib.o
gcc -c src/main.c -o build/main.o
gcc build/mylib.o build/main.o -o myprogram
```

### C Usage Example

```c
// main.c
#include "mylib.h"
#include <stdio.h>

int main() {
    int result = calculate(10, 20);
    printf("Result: %d\n", result);
    return 0;
}
```

## Best Practices

1. **Use @repr(C)** - Always for structs passed to/from C
2. **Keep public API simple** - Avoid exposing opt/res types
3. **Document ownership** - Specify who allocates/frees memory
4. **Use integer error codes** - C idiom for error handling
5. **Provide cleanup functions** - For any allocated resources
6. **Keep headers minimal** - Only expose what's necessary

## Library Project Structure

```
mylib/
├── fastc.toml
├── src/
│   ├── lib.fc          # Public API
│   └── internal.fc     # Private helpers
├── include/
│   └── mylib.h         # Generated header
└── build/
    └── mylib.c         # Generated source
```

## See Also

- [Calling C](calling-c.md) - Using C from FastC
- [Build Systems](build-systems.md) - Integration with build tools
- [Structs & Enums](../language/structs-enums.md) - Type definitions
