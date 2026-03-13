# Tutorial Examples

Work through these tutorials in order to learn FastC fundamentals.

## 01: Hello World

The minimal FastC program:

```c
// Every FastC program needs main() returning i32
fn main() -> i32 {
    return 0;  // Exit code 0 = success
}
```

**Key concepts:**

- `fn` declares a function
- `main` is the entry point
- `-> i32` specifies return type
- `return` exits with a value

## 02: Variables and Types

Variables are declared with `let` and require type annotations:

```c
fn main() -> i32 {
    // Integer types
    let integer: i32 = 42;
    let negative: i32 = -100;

    // Floating point
    let precise: f64 = 2.718281828459045;

    // Boolean
    let flag: bool = true;

    // Arithmetic
    let sum: i32 = (integer + 10);
    let product: i32 = (integer * 2);
    let quotient: i32 = (integer / 2);
    let remainder: i32 = (integer % 5);

    // Comparison returns bool
    let is_greater: bool = (integer > 10);

    return sum;
}
```

**Key concepts:**

- `let` declares variables
- Explicit type annotations: `name: type`
- Primitive types: `i32`, `f64`, `bool`
- Arithmetic and comparison operators

## 03: Functions

Functions with parameters and return values:

```c
// Function with parameters
fn add(a: i32, b: i32) -> i32 {
    return (a + b);
}

// Function without return value
fn greet() {
    // Do something
}

// Multiple statements
fn calculate(x: i32) -> i32 {
    let doubled: i32 = (x * 2);
    let result: i32 = (doubled + 10);
    return result;
}

fn main() -> i32 {
    let sum: i32 = add(10, 20);
    return sum;
}
```

**Key concepts:**

- Parameters with types: `name: type`
- Return type after `->`
- Functions without return omit `-> type`
- Call functions with `name(args)`

## 04: Control Flow

Conditionals and loops:

```c
fn main() -> i32 {
    let x: i32 = 10;

    // if-else
    if (x > 5) {
        x = (x + 1);
    } else {
        x = (x - 1);
    }

    // while loop
    let i: i32 = 0;
    while (i < 10) {
        i = (i + 1);
    }

    // for loop
    for let j: i32 = 0; j < 5; j = j + 1 {
        discard(j);
    }

    // switch statement
    let day: i32 = 3;
    switch (day) {
        case 1: { x = 10; }
        case 2: { x = 20; }
        case 3: { x = 30; }
        default: { x = 0; }
    }

    return x;
}
```

**Key concepts:**

- `if`/`else` with parenthesized conditions
- `while` loops
- `for` loops with init, condition, update
- `switch` with `case` and `default`

## 05: Arrays and Slices

Fixed-size arrays and dynamic slices:

```c
fn main() -> i32 {
    // Fixed-size array
    let numbers: arr(i32, 5) = [1, 2, 3, 4, 5];

    // Access elements with at()
    let first: i32 = at(numbers, 0);
    let third: i32 = at(numbers, 2);

    // Get length
    let size: usize = len(numbers);

    // Slice from array
    let s: slice(i32) = slice_from(numbers);

    // Access slice elements
    let elem: i32 = at(s, 1);

    discard(first);
    discard(third);
    discard(size);
    discard(elem);

    return 0;
}
```

**Key concepts:**

- `arr(T, N)` for fixed-size arrays
- `slice(T)` for dynamic views
- `at(array, index)` for element access
- `len(array)` for length
- `slice_from(array)` to create slice

## 06: Pointers

References and raw pointers:

```c
fn increment(x: mref(i32)) {
    deref(x) = (deref(x) + 1);
}

fn read_value(x: ref(i32)) -> i32 {
    return deref(x);
}

fn main() -> i32 {
    let value: i32 = 10;

    // Create mutable reference
    let ptr: mref(i32) = addr(value);

    // Modify through reference
    increment(ptr);  // value is now 11

    // Read-only reference
    let rptr: ref(i32) = addr(value);
    let result: i32 = read_value(rptr);

    return result;
}
```

**Key concepts:**

- `ref(T)` - read-only reference
- `mref(T)` - mutable reference
- `addr(x)` - take address
- `deref(p)` - dereference pointer
- `raw(T)` and `rawm(T)` for unsafe pointers

## 07: Optionals

Safe handling of optional values:

```c
fn find_positive(value: i32) -> opt(i32) {
    if (value > 0) {
        return some(value);
    } else {
        return none(i32);
    }
}

fn safe_divide(a: i32, b: i32) -> opt(i32) {
    if (b == 0) {
        return none(i32);
    }
    return some(a / b);
}

fn main() -> i32 {
    let result: opt(i32) = find_positive(42);

    // Use if-let to safely unwrap
    if let value = unwrap_checked(result) {
        return value;  // 42
    } else {
        return -1;  // Handle none case
    }
}
```

**Key concepts:**

- `opt(T)` for optional values
- `some(value)` creates present optional
- `none(T)` creates absent optional
- `if let` safely unwraps optionals
- `unwrap_checked` in if-let context

## 08: Results

Error handling with result types:

```c
enum MathError {
    DivisionByZero,
    Overflow,
    InvalidInput,
}

fn safe_divide(a: i32, b: i32) -> res(i32, MathError) {
    if (b == 0) {
        return err(MathError_DivisionByZero);
    }
    return ok(a / b);
}

fn main() -> i32 {
    let result: res(i32, MathError) = safe_divide(10, 2);

    if is_ok(result) {
        return unwrap(result);  // 5
    } else {
        return -1;  // Handle error
    }
}
```

**Key concepts:**

- `res(T, E)` for result types
- `ok(value)` creates success result
- `err(error)` creates error result
- `is_ok(result)` checks for success
- `unwrap(result)` extracts value

## 09: Structs

User-defined data types:

```c
struct Point {
    x: f64,
    y: f64,
}

fn create_point(x: f64, y: f64) -> Point {
    return Point { x: x, y: y };
}

fn distance(p: ref(Point)) -> f64 {
    let dx: f64 = deref(p).x;
    let dy: f64 = deref(p).y;
    return sqrt(dx * dx + dy * dy);
}

fn main() -> i32 {
    let p: Point = create_point(3.0, 4.0);

    // Access fields
    let x_val: f64 = p.x;
    let y_val: f64 = p.y;

    discard(x_val);
    discard(y_val);

    return 0;
}
```

**Key concepts:**

- `struct Name { fields }` defines types
- `Name { field: value }` creates instances
- `instance.field` accesses fields
- `@repr(C)` for C-compatible layout

## 10: Enums

Enumerated types:

```c
enum Color {
    Red,
    Green,
    Blue,
}

enum Status {
    Pending,
    Running,
    Complete,
    Failed,
}

fn get_priority(status: Status) -> i32 {
    switch (status) {
        case Status_Running: { return 1; }
        case Status_Pending: { return 2; }
        case Status_Complete: { return 3; }
        case Status_Failed: { return 0; }
    }
}

fn main() -> i32 {
    let color: Color = Color_Red;
    let status: Status = Status_Running;

    let priority: i32 = get_priority(status);

    discard(color);

    return priority;
}
```

**Key concepts:**

- `enum Name { Variant1, Variant2 }` defines enums
- `EnumName_Variant` accesses variants
- `switch` for enum matching
- All cases should be handled

## Running Tutorials

```bash
# Compile and run a tutorial
fastc compile examples/tutorials/01_hello_world.fc -o /tmp/hello.c
cc /tmp/hello.c -o /tmp/hello
/tmp/hello
echo $?  # Check exit code
```

## Next Steps

- [Advanced Examples](advanced.md) - Real-world patterns
- [Language Guide](../language/index.md) - Complete reference

