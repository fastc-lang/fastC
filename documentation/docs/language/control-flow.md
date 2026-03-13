# Control Flow

FastC provides familiar control flow constructs for conditionals and loops.

## If Statements

### Basic If

```c
if condition {
    // executed if condition is true
}
```

### If-Else

```c
if condition {
    // executed if true
} else {
    // executed if false
}
```

### If-Else If-Else

```c
if condition1 {
    // ...
} else if condition2 {
    // ...
} else {
    // ...
}
```

### Example

```c
fn classify(n: i32) -> i32 {
    if n < 0 {
        return -1;
    } else if n == 0 {
        return 0;
    } else {
        return 1;
    }
}
```

## If-Let (Optional Unwrapping)

Unwrap optional values safely:

```c
fn process(maybe_value: opt(i32)) -> i32 {
    if let value = maybe_value {
        // value is i32 here, not opt(i32)
        return value * 2;
    } else {
        return 0;
    }
}
```

See [Optionals](optionals.md) for more details.

## While Loops

```c
while condition {
    // loop body
}
```

### Example

```c
fn countdown(start: i32) {
    let n: i32 = start;
    while n > 0 {
        // do something
        n = n - 1;
    }
}
```

## For Loops

FastC uses C-style for loops:

```c
for init; condition; update {
    // loop body
}
```

### Example

```c
fn sum_to_n(n: i32) -> i32 {
    let total: i32 = 0;
    for let i: i32 = 1; i <= n; i = i + 1 {
        total = total + i;
    }
    return total;
}
```

### Iterating Over a Range

```c
for let i: i32 = 0; i < 10; i = i + 1 {
    // i goes from 0 to 9
}
```

### Countdown

```c
for let i: i32 = 10; i > 0; i = i - 1 {
    // i goes from 10 to 1
}
```

## Break and Continue

### Break

Exit a loop early:

```c
fn find_first_negative(data: slice(i32)) -> i32 {
    let result: i32 = -1;
    for let i: i32 = 0; i < len(data); i = i + 1 {
        if at(data, i) < 0 {
            result = at(data, i);
            break;
        }
    }
    return result;
}
```

### Continue

Skip to the next iteration:

```c
fn sum_positive(data: slice(i32)) -> i32 {
    let total: i32 = 0;
    for let i: i32 = 0; i < len(data); i = i + 1 {
        if at(data, i) < 0 {
            continue;
        }
        total = total + at(data, i);
    }
    return total;
}
```

## Switch Statements

Match a value against multiple cases:

```c
switch expression {
    case value1:
        // ...
    case value2:
        // ...
    default:
        // ...
}
```

### Example

```c
fn day_name(day: i32) -> i32 {
    switch day {
        case 1:
            return 1;  // Monday
        case 2:
            return 2;  // Tuesday
        case 3:
            return 3;  // Wednesday
        default:
            return 0;  // Unknown
    }
}
```

### With Enums

```c
enum Status {
    Pending,
    Running,
    Complete,
    Failed,
}

fn handle_status(status: Status) -> i32 {
    switch status {
        case Status::Pending:
            return 0;
        case Status::Running:
            return 1;
        case Status::Complete:
            return 2;
        case Status::Failed:
            return -1;
        default:
            return -2;
    }
}
```

## Return Statements

Exit a function and optionally return a value:

```c
fn early_return(x: i32) -> i32 {
    if x < 0 {
        return -1;  // Early return
    }

    // Normal processing
    return x * 2;
}
```

### Void Functions

Use `return;` without a value:

```c
fn maybe_log(should_log: bool) {
    if !should_log {
        return;  // Early exit
    }
    // Do logging
}
```

## Conditions Must Be Boolean

Unlike C, conditions must be explicitly boolean:

```c
let x: i32 = 5;

// Error: i32 is not bool
if x {
    // ...
}

// Correct: explicit comparison
if x != 0 {
    // ...
}
```

## Short-Circuit Evaluation

Logical operators `&&` and `||` short-circuit:

```c
// b() is only called if a() returns true
if a() && b() {
    // ...
}

// b() is only called if a() returns false
if a() || b() {
    // ...
}
```

This is important for avoiding null checks:

```c
if is_valid(ptr) && check_value(ptr) {
    // check_value is only called if is_valid returns true
}
```
