# Runtime Reference

The FastC runtime header (`fastc_runtime.h`) provides essential functions and types for generated C code.

## Overview

The runtime is a lightweight header-only library that provides:

- Trap handler for safety violations
- Memory allocation stubs
- Unaligned memory access helpers
- Slice type definitions

## Including the Runtime

Generated C code includes the runtime:

```c
#include "fastc_runtime.h"
```

When compiling, provide the include path:

```bash
cc -I /path/to/fastc/runtime generated.c -o program
```

## Trap Handler

```c
static inline _Noreturn void fc_trap(void);
```

Called when a safety violation occurs:

- Array bounds check failure
- Null pointer dereference (in checked contexts)
- Arithmetic overflow (when enabled)

Default behavior calls `abort()`. Replace for custom handling.

## Memory Allocation

```c
static inline void* fc_alloc(size_t size, size_t align);
static inline void fc_free(void* ptr);
```

Default implementations use `malloc()` and `free()`. Replace for:

- Custom allocators
- Arena/pool allocation
- Debugging/instrumentation

### Customization Example

```c
// Before including runtime
#define FC_CUSTOM_ALLOC

static inline void* fc_alloc(size_t size, size_t align) {
    // Custom allocation logic
    return my_allocator_alloc(size, align);
}

static inline void fc_free(void* ptr) {
    my_allocator_free(ptr);
}

#include "fastc_runtime.h"
```

## Memory Operations

```c
static inline void fc_memcpy(void* dst, const void* src, size_t n);
```

Byte-by-byte memory copy. Used internally for safe unaligned access.

## Unaligned Access Helpers

Safe unaligned memory reads and writes:

### Read Functions

```c
static inline uint16_t fc_read_u16_unaligned(const void* ptr);
static inline uint32_t fc_read_u32_unaligned(const void* ptr);
static inline uint64_t fc_read_u64_unaligned(const void* ptr);
```

### Write Functions

```c
static inline void fc_write_u16_unaligned(void* ptr, uint16_t val);
static inline void fc_write_u32_unaligned(void* ptr, uint32_t val);
static inline void fc_write_u64_unaligned(void* ptr, uint64_t val);
```

These avoid undefined behavior from unaligned pointer casts.

## Slice Types

The macro `FC_DEFINE_SLICE` creates slice types:

```c
#define FC_DEFINE_SLICE(T, name) \
    typedef struct { T* data; size_t len; } name
```

### Predefined Slice Types

| FastC Type | C Slice Type |
|------------|--------------|
| `slice(u8)` | `fc_slice_uint8_t` |
| `slice(i8)` | `fc_slice_int8_t` |
| `slice(u16)` | `fc_slice_uint16_t` |
| `slice(i16)` | `fc_slice_int16_t` |
| `slice(u32)` | `fc_slice_uint32_t` |
| `slice(i32)` | `fc_slice_int32_t` |
| `slice(u64)` | `fc_slice_uint64_t` |
| `slice(i64)` | `fc_slice_int64_t` |
| `slice(f32)` | `fc_slice_float` |
| `slice(f64)` | `fc_slice_double` |

### Custom Slice Types

For user-defined types:

```c
// In FastC
struct Point { x: f64, y: f64 }
// Slice would be generated as needed

// In C, manually:
FC_DEFINE_SLICE(Point, fc_slice_Point);
```

## Slice Structure

All slices have the same layout:

```c
struct {
    T* data;      // Pointer to first element
    size_t len;   // Number of elements
};
```

## Runtime Requirements

The runtime depends on standard C headers:

- `<stddef.h>` - `size_t`, `NULL`
- `<stdint.h>` - Fixed-width integers
- `<stdbool.h>` - `bool` type
- `<stdlib.h>` - `malloc`, `free`, `abort`

## Compiler Compatibility

The runtime is compatible with:

- GCC 4.6+
- Clang 3.0+
- MSVC 2015+

Uses C11 features:

- `_Noreturn` function specifier
- `<stdbool.h>` boolean type

## Full Header

```c
/* FastC Runtime Header */
#ifndef FASTC_RUNTIME_H
#define FASTC_RUNTIME_H

#include <stddef.h>
#include <stdint.h>
#include <stdbool.h>
#include <stdlib.h>

/* Trap handler - abort on safety violation */
static inline _Noreturn void fc_trap(void) {
    abort();
}

/* Allocator stubs - users may replace */
static inline void* fc_alloc(size_t size, size_t align) {
    (void)align;
    return malloc(size);
}

static inline void fc_free(void* ptr) {
    free(ptr);
}

/* Memory copy */
static inline void fc_memcpy(void* dst, const void* src, size_t n) {
    unsigned char* d = (unsigned char*)dst;
    const unsigned char* s = (const unsigned char*)src;
    while (n--) {
        *d++ = *s++;
    }
}

/* Unaligned read helpers */
static inline uint16_t fc_read_u16_unaligned(const void* ptr) {
    uint16_t val;
    fc_memcpy(&val, ptr, sizeof(val));
    return val;
}

static inline uint32_t fc_read_u32_unaligned(const void* ptr) {
    uint32_t val;
    fc_memcpy(&val, ptr, sizeof(val));
    return val;
}

static inline uint64_t fc_read_u64_unaligned(const void* ptr) {
    uint64_t val;
    fc_memcpy(&val, ptr, sizeof(val));
    return val;
}

/* Unaligned write helpers */
static inline void fc_write_u16_unaligned(void* ptr, uint16_t val) {
    fc_memcpy(ptr, &val, sizeof(val));
}

static inline void fc_write_u32_unaligned(void* ptr, uint32_t val) {
    fc_memcpy(ptr, &val, sizeof(val));
}

static inline void fc_write_u64_unaligned(void* ptr, uint64_t val) {
    fc_memcpy(ptr, &val, sizeof(val));
}

/* Slice type macro */
#define FC_DEFINE_SLICE(T, name) \
    typedef struct { T* data; size_t len; } name

/* Common slice types */
FC_DEFINE_SLICE(uint8_t, fc_slice_uint8_t);
FC_DEFINE_SLICE(int8_t, fc_slice_int8_t);
FC_DEFINE_SLICE(uint16_t, fc_slice_uint16_t);
FC_DEFINE_SLICE(int16_t, fc_slice_int16_t);
FC_DEFINE_SLICE(uint32_t, fc_slice_uint32_t);
FC_DEFINE_SLICE(int32_t, fc_slice_int32_t);
FC_DEFINE_SLICE(uint64_t, fc_slice_uint64_t);
FC_DEFINE_SLICE(int64_t, fc_slice_int64_t);
FC_DEFINE_SLICE(float, fc_slice_float);
FC_DEFINE_SLICE(double, fc_slice_double);

#endif /* FASTC_RUNTIME_H */
```

## See Also

- [Installation](../getting-started/installation.md) - Finding runtime path
- [Build Systems](../c-interop/build-systems.md) - Setting include paths
- [Unsafe Code](../language/unsafe.md) - When runtime functions are used

