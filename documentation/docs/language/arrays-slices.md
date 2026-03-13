# Arrays and Slices

FastC provides two collection types: fixed-size arrays and dynamic slices.

## Arrays - arr(T, N)

Arrays have a fixed size known at compile time.

### Declaration

```c
let numbers: arr(i32, 5) = [1, 2, 3, 4, 5];
let zeros: arr(i32, 10) = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
```

### Size in Type

The size is part of the type:

```c
fn process_five(data: arr(i32, 5)) {
    // Only accepts arrays of exactly 5 elements
}

fn process_ten(data: arr(i32, 10)) {
    // Only accepts arrays of exactly 10 elements
}
```

### Constant Size Expressions

Array sizes can use constant expressions:

```c
const SIZE: i32 = 100;
let buffer: arr(u8, SIZE) = /* ... */;
let matrix: arr(i32, 4 * 4) = /* ... */;
```

## Slices - slice(T)

Slices are views into contiguous memory with a runtime-known length.

### Structure

A slice contains:

- A pointer to the data
- A length

```c
struct Slice {
    data: rawm(T),
    len: usize,
}
```

### Creating Slices

From string literals:

```c
let message: slice(u8) = c"Hello, world!";
```

From arrays (future feature):

```c
// Convert array to slice
let arr: arr(i32, 5) = [1, 2, 3, 4, 5];
let s: slice(i32) = slice_from_array(addr(arr), 5);
```

### Getting Length

```c
fn print_length(data: slice(i32)) {
    let length: usize = len(data);
    // Use length...
}
```

## Element Access - at()

Use `at()` for bounds-checked element access:

```c
fn sum(data: slice(i32)) -> i32 {
    let total: i32 = 0;
    for let i: i32 = 0; i < len(data); i = i + 1 {
        total = total + at(data, i);
    }
    return total;
}
```

### Bounds Checking

`at()` performs runtime bounds checking in safe code:

```c
let data: slice(i32) = get_data();
let value: i32 = at(data, 100);  // Traps if len(data) <= 100
```

### Mutable Access

```c
fn zero_slice(data: slice(i32)) {
    for let i: i32 = 0; i < len(data); i = i + 1 {
        at(data, i) = 0;  // Write to element
    }
}
```

## Common Operations

### Iteration

```c
fn find_max(data: slice(i32)) -> i32 {
    let max: i32 = at(data, 0);
    for let i: i32 = 1; i < len(data); i = i + 1 {
        if at(data, i) > max {
            max = at(data, i);
        }
    }
    return max;
}
```

### Search

```c
fn contains(data: slice(i32), target: i32) -> bool {
    for let i: i32 = 0; i < len(data); i = i + 1 {
        if at(data, i) == target {
            return true;
        }
    }
    return false;
}

fn find_index(data: slice(i32), target: i32) -> opt(i32) {
    for let i: i32 = 0; i < len(data); i = i + 1 {
        if at(data, i) == target {
            return some(i);
        }
    }
    return none(i32);
}
```

### Copy

```c
fn copy_slice(dest: slice(i32), src: slice(i32)) {
    let count: i32 = min(len(dest), len(src));
    for let i: i32 = 0; i < count; i = i + 1 {
        at(dest, i) = at(src, i);
    }
}
```

### Reverse

```c
fn reverse(data: slice(i32)) {
    let left: i32 = 0;
    let right: i32 = len(data) - 1;
    while left < right {
        let temp: i32 = at(data, left);
        at(data, left) = at(data, right);
        at(data, right) = temp;
        left = left + 1;
        right = right - 1;
    }
}
```

## Generated C Code

### Slice Structure

A `slice(i32)` compiles to:

```c
typedef struct fc_slice_int32_t {
    int32_t* data;
    size_t len;
} fc_slice_int32_t;
```

### Bounds Checking

`at(data, i)` compiles to:

```c
// With bounds check (safe code)
if ((size_t)i >= data.len) {
    fc_trap();
}
data.data[i]
```

## C Interoperability

### Receiving Slices from C

```c
extern "C" {
    fn get_buffer(out_ptr: mref(rawm(u8)), out_len: mref(usize));
}

fn get_data() -> slice(u8) {
    let ptr: rawm(u8) = null;
    let length: usize = 0;

    unsafe {
        get_buffer(addr(ptr), addr(length));
    }

    // Construct slice
    return make_slice(ptr, length);
}
```

### Passing Slices to C

```c
extern "C" {
    fn process_buffer(data: rawm(u8), len: usize);
}

fn send_data(data: slice(u8)) {
    unsafe {
        process_buffer(data.data, data.len);
    }
}
```

## Best Practices

1. **Use slices for parameters** - More flexible than arrays
2. **Check length before access** - Avoid bounds check overhead in loops
3. **Prefer at() over raw pointers** - Automatic bounds checking
4. **Use arrays for fixed-size data** - Stack allocated, no indirection
5. **Document slice ownership** - Who frees the underlying memory?

## See Also

- [Types](types.md) - Type system overview
- [Pointers](pointers.md) - Raw pointer access
- [C Interoperability](../c-interop/calling-c.md) - Passing slices to C
