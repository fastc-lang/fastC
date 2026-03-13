# Optionals

Optionals represent values that may or may not be present. They eliminate null pointer errors by making absence explicit.

## The opt(T) Type

An `opt(T)` can be either:

- `some(value)` - Contains a value of type T
- `none(T)` - Contains no value

```c
let present: opt(i32) = some(42);
let absent: opt(i32) = none(i32);
```

## Creating Optionals

### some(value)

Wrap a value in an optional:

```c
let x: opt(i32) = some(10);
let y: opt(f64) = some(3.14);
```

### none(T)

Create an empty optional:

```c
let empty: opt(i32) = none(i32);
let no_data: opt(Point) = none(Point);
```

## Checking for Values

### Using if-let

The safest way to unwrap an optional:

```c
fn process(maybe_value: opt(i32)) -> i32 {
    if let value = maybe_value {
        // 'value' is i32 here, guaranteed to exist
        return value * 2;
    } else {
        // Handle the none case
        return 0;
    }
}
```

### Example: Safe Division

```c
fn safe_divide(a: i32, b: i32) -> opt(i32) {
    if b == 0 {
        return none(i32);
    }
    return some(a / b);
}

fn main() -> i32 {
    let result: opt(i32) = safe_divide(10, 2);
    if let value = result {
        return value;  // Returns 5
    }
    return -1;  // Division failed
}
```

## Patterns with Optionals

### Early Return Pattern

```c
fn find_user(id: i32) -> opt(User) {
    // ... lookup user ...
    if !found {
        return none(User);
    }
    return some(user);
}

fn get_user_name(id: i32) -> opt(slice(u8)) {
    if let user = find_user(id) {
        return some(user.name);
    }
    return none(slice(u8));
}
```

### Default Value Pattern

```c
fn get_or_default(maybe: opt(i32), default: i32) -> i32 {
    if let value = maybe {
        return value;
    }
    return default;
}

fn main() -> i32 {
    let x: opt(i32) = none(i32);
    return get_or_default(x, 42);  // Returns 42
}
```

### Transform Pattern

```c
fn double_if_present(maybe: opt(i32)) -> opt(i32) {
    if let value = maybe {
        return some(value * 2);
    }
    return none(i32);
}
```

## Common Use Cases

### Optional Function Parameters

```c
fn format_number(n: i32, prefix: opt(slice(u8))) -> slice(u8) {
    if let p = prefix {
        // Use prefix
    }
    // Format without prefix
}
```

### Optional Struct Fields

```c
struct Config {
    host: slice(u8),
    port: i32,
    timeout: opt(i32),  // Optional timeout
}
```

### Lookup Functions

```c
fn find_index(data: slice(i32), target: i32) -> opt(i32) {
    for let i: i32 = 0; i < len(data); i = i + 1 {
        if at(data, i) == target {
            return some(i);
        }
    }
    return none(i32);
}
```

## Generated C Code

An `opt(i32)` compiles to a struct:

```c
typedef struct fc_opt_int32_t {
    bool has_value;
    int32_t value;
} fc_opt_int32_t;
```

Creating values:

```c
// some(42) becomes:
(fc_opt_int32_t){ .has_value = true, .value = 42 }

// none(i32) becomes:
(fc_opt_int32_t){ .has_value = false }
```

## Best Practices

1. **Prefer optionals over sentinel values** - Use `opt(i32)` instead of `-1` for "not found"
2. **Always use if-let** - Don't access `.value` directly without checking
3. **Return early with none** - Makes error paths clear
4. **Document when functions return none** - Explain the conditions

## Comparison with C

| C Pattern | FastC Pattern |
|-----------|---------------|
| `return NULL` | `return none(T)` |
| `if (ptr != NULL)` | `if let value = opt` |
| `-1` for "not found" | `none(i32)` |
| Out parameters | Return `opt(T)` |

## See Also

- [Results](results.md) - For operations that can fail with error info
- [Pointers](pointers.md) - For nullable pointers
