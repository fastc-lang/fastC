# Results

Result types represent operations that can succeed or fail, carrying either a success value or an error.

## The res(T, E) Type

A `res(T, E)` can be either:

- `ok(value)` - Operation succeeded with value of type T
- `err(error)` - Operation failed with error of type E

```c
let success: res(i32, Error) = ok(42);
let failure: res(i32, Error) = err(Error::NotFound);
```

## Defining Error Types

Use enums to define error types:

```c
enum ParseError {
    InvalidFormat,
    OutOfRange,
    UnexpectedEnd,
}

enum IoError {
    NotFound,
    PermissionDenied,
    ConnectionReset,
}
```

## Creating Results

### ok(value)

Return a success value:

```c
fn parse_positive(s: slice(u8)) -> res(i32, ParseError) {
    // ... parsing logic ...
    if valid {
        return ok(number);
    }
    return err(ParseError::InvalidFormat);
}
```

### err(error)

Return an error:

```c
fn divide(a: i32, b: i32) -> res(i32, slice(u8)) {
    if b == 0 {
        return err(c"division by zero");
    }
    return ok(a / b);
}
```

## Handling Results

### Check and Extract

```c
fn process_result(r: res(i32, Error)) -> i32 {
    if r.is_ok {
        return r.ok;  // Access success value
    } else {
        // Handle error
        return -1;
    }
}
```

### Pattern: Early Return

```c
fn complex_operation() -> res(i32, Error) {
    let step1: res(i32, Error) = first_step();
    if !step1.is_ok {
        return step1;  // Propagate error
    }

    let step2: res(i32, Error) = second_step(step1.ok);
    if !step2.is_ok {
        return step2;  // Propagate error
    }

    return ok(step2.ok * 2);
}
```

## Common Patterns

### Validation

```c
enum ValidationError {
    TooShort,
    TooLong,
    InvalidChar,
}

fn validate_username(name: slice(u8)) -> res(slice(u8), ValidationError) {
    if len(name) < 3 {
        return err(ValidationError::TooShort);
    }
    if len(name) > 20 {
        return err(ValidationError::TooLong);
    }
    // ... more validation ...
    return ok(name);
}
```

### File Operations

```c
enum FileError {
    NotFound,
    PermissionDenied,
    IoError,
}

fn read_config() -> res(Config, FileError) {
    // ... file reading logic ...
    if !file_exists {
        return err(FileError::NotFound);
    }
    return ok(config);
}
```

### Network Operations

```c
enum NetError {
    ConnectionFailed,
    Timeout,
    InvalidResponse,
}

fn fetch_data(url: slice(u8)) -> res(slice(u8), NetError) {
    // ... network logic ...
    if timeout {
        return err(NetError::Timeout);
    }
    return ok(response);
}
```

## Combining Results

### Sequential Operations

```c
fn process_pipeline(input: i32) -> res(i32, Error) {
    // Step 1
    let r1: res(i32, Error) = step_one(input);
    if !r1.is_ok {
        return r1;
    }

    // Step 2
    let r2: res(i32, Error) = step_two(r1.ok);
    if !r2.is_ok {
        return r2;
    }

    // Step 3
    return step_three(r2.ok);
}
```

### Converting Errors

```c
enum HighLevelError {
    ParseFailed,
    ValidationFailed,
    IoFailed,
}

fn load_and_validate(path: slice(u8)) -> res(Data, HighLevelError) {
    let file_result: res(slice(u8), IoError) = read_file(path);
    if !file_result.is_ok {
        return err(HighLevelError::IoFailed);
    }

    let parse_result: res(Data, ParseError) = parse_data(file_result.ok);
    if !parse_result.is_ok {
        return err(HighLevelError::ParseFailed);
    }

    return ok(parse_result.ok);
}
```

## Generated C Code

A `res(i32, Error)` compiles to a struct:

```c
typedef struct fc_res_int32_t_Error {
    bool is_ok;
    int32_t ok;
    Error err;
} fc_res_int32_t_Error;
```

Creating values:

```c
// ok(42) becomes:
(fc_res_int32_t_Error){ .is_ok = true, .ok = 42 }

// err(Error::NotFound) becomes:
(fc_res_int32_t_Error){ .is_ok = false, .err = Error_NotFound }
```

## Best Practices

1. **Use descriptive error types** - Create specific error enums
2. **Don't ignore errors** - Always check `.is_ok` before using `.ok`
3. **Propagate errors early** - Return errors as soon as they occur
4. **Document error conditions** - Explain when each error variant is returned
5. **Use results for recoverable errors** - Use traps for programming errors

## Result vs Optional

| Use Case | Type |
|----------|------|
| Value might not exist | `opt(T)` |
| Operation can fail | `res(T, E)` |
| Lookup might fail | `opt(T)` (no error info needed) |
| Parsing might fail | `res(T, ParseError)` (error info useful) |

## See Also

- [Optionals](optionals.md) - For values that may be absent
- [Unsafe Code](unsafe.md) - For error handling in unsafe contexts
