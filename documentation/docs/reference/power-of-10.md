# NASA/JPL Power of 10: Safety-Critical Code

FastC implements NASA/JPL's "Power of 10" rules for developing safety-critical code, making it suitable for aerospace, medical, automotive, and other high-reliability applications.

## Introduction

The Power of 10 rules were developed by Gerard J. Holzmann at NASA's Jet Propulsion Laboratory for the Mars Science Laboratory mission. These rules are designed to make critical software more analyzable and verifiable.

**Key principle**: When human lives depend on software correctness, stricter coding standards are worth the extra effort.

FastC enforces these rules automatically through static analysis, making safety-critical development accessible without manual code reviews.

## Safety Levels

FastC provides three safety levels:

| Level | Description | Use Case |
|-------|-------------|----------|
| **Standard** | Rules 2, 3, 4, 9 enabled | General development (default) |
| **Critical** | All 10 rules enabled | Safety-critical systems |
| **Relaxed** | No P10 rules | Prototyping only |

```bash
# Standard (default) - key safety rules enabled
fastc check src/main.fc

# Critical - full Power of 10 compliance
fastc check --safety-level=critical src/main.fc

# Relaxed - disable for prototyping
fastc check --safety-level=relaxed src/main.fc
```

## Rule Enforcement Summary

| Rule | Description | Standard | Critical |
|------|-------------|:--------:|:--------:|
| 1 | No recursion | - | Yes |
| 2 | Bounded loops | Yes | Yes |
| 3 | No dynamic allocation | Yes | Yes |
| 4 | Function size limit | Yes | Yes |
| 5 | Assertion density | Planned | Planned |
| 6 | Minimal scope | By design | By design |
| 7 | Check return values | By design | By design |
| 8 | Limited preprocessor | By design | By design |
| 9 | Restricted pointers | Yes | Yes |
| 10 | Zero warnings | --strict | --strict |

---

## The 10 Rules

### Rule 1: Simple Control Flow

> "Restrict all code to very simple control flow constructs—do not use goto statements, setjmp or longjmp constructs, or direct or indirect recursion."

**Rationale**: Simpler control flow enables stronger static analysis and often results in clearer code. An acyclic call graph allows analyzers to prove bounds on stack use and execution time.

**FastC Implementation**:
- No `goto` keyword in the language
- No `setjmp`/`longjmp` constructs
- Recursion detection via call graph analysis (Critical mode)

```c
// VIOLATION: Recursive function (detected in Critical mode)
fn factorial(n: i32) -> i32 {
    if n <= 1 {
        return 1;
    }
    return n * factorial(n - 1);  // Error: recursive call
}

// COMPLIANT: Iterative version
fn factorial(n: i32) -> i32 {
    let result: i32 = 1;
    for let i: i32 = 2; i <= n; i = i + 1 {
        result = result * i;
    }
    return result;
}
```

---

### Rule 2: Bounded Loops

> "Give all loops a fixed upper bound. It must be trivially possible for a checking tool to prove statically that the loop cannot exceed a preset upper bound."

**Rationale**: Combined with no recursion, bounded loops prevent runaway code and enable termination proofs.

**FastC Implementation**:
- Detects `while(true)` as unbounded
- Flags `for` loops without conditions
- Requires provable termination

```c
// VIOLATION: Unbounded loop
fn bad_loop() {
    while (true) {  // Error: no provable upper bound
        // ...
    }
}

// COMPLIANT: Bounded loop
fn good_loop(data: slice(i32)) {
    for let i: usize = 0; i < len(data); i = i + 1 {
        // Provably bounded by data length
    }
}

// COMPLIANT: Loop with explicit bound
fn process_with_limit() {
    let iterations: i32 = 0;
    let max_iterations: i32 = 1000;

    while (iterations < max_iterations) {
        // Bounded by max_iterations
        iterations = iterations + 1;
    }
}
```

---

### Rule 3: No Dynamic Memory After Initialization

> "Do not use dynamic memory allocation after initialization."

**Rationale**: Memory allocators have unpredictable behavior affecting real-time performance. Many bugs stem from mishandling allocation: leaks, use-after-free, double-free, and buffer overruns.

**FastC Implementation**:
- Flags calls to `malloc`, `calloc`, `realloc`, `free`
- Encourages stack allocation and pre-allocated buffers

```c
// VIOLATION: Runtime allocation
unsafe fn bad_alloc() {
    extern "C" { fn malloc(size: usize) -> rawm(u8); }
    let ptr: rawm(u8) = malloc(100);  // Error: dynamic allocation
}

// COMPLIANT: Stack allocation
fn good_alloc() {
    let buffer: arr(u8, 100) = [0; 100];  // Fixed-size stack allocation
}

// COMPLIANT: Pre-allocated buffer passed in
fn process(buffer: slice(u8)) {
    // Work with caller-provided memory
}
```

---

### Rule 4: Function Size Limit

> "No function should be longer than what can be printed on a single sheet of paper—about 60 lines of code."

**Rationale**: Each function should be a logical unit that is understandable and verifiable. Long functions often indicate poor structure.

**FastC Implementation**:
- Enforces configurable line limit (default: 60)
- Counts non-empty, non-comment lines

```c
// VIOLATION: Function too long (>60 lines)
fn overly_complex() -> i32 {
    // ... 70+ lines of code ...
}  // Error: function exceeds 60-line limit

// COMPLIANT: Decomposed into focused functions
fn validate_input(data: slice(i32)) -> bool {
    // 15 lines
}

fn process_data(data: slice(i32)) -> i32 {
    // 20 lines
}

fn format_output(result: i32) -> i32 {
    // 10 lines
}

fn main() -> i32 {
    let data: arr(i32, 10) = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    if validate_input(slice_from(data)) {
        let result: i32 = process_data(slice_from(data));
        return format_output(result);
    }
    return 1;
}
```

---

### Rule 5: Assertion Density

> "The code's assertion density should average minimally two assertions per function."

**Rationale**: Assertions verify pre/postconditions, parameter values, and invariants. Higher assertion density increases defect detection.

**FastC Implementation**: *Planned feature*

When implemented, FastC will require:
- Minimum 2 assertions per function in Critical mode
- Side-effect free assertion expressions
- Explicit recovery on assertion failure

```c
// Future syntax (planned)
fn divide(a: i32, b: i32) -> i32 {
    assert(b != 0, "divisor must be non-zero");
    assert(a >= 0, "dividend must be non-negative");
    return a / b;
}
```

---

### Rule 6: Minimal Scope

> "Declare all data objects at the smallest possible level of scope."

**Rationale**: Limited scope prevents accidental corruption, simplifies debugging, and discourages variable reuse for incompatible purposes.

**FastC Implementation**: Enforced by language design
- Block-scoped `let` declarations
- No global mutable state in safe code
- Variables must be initialized at declaration

```c
fn example(items: slice(i32)) -> i32 {
    let total: i32 = 0;

    for let i: usize = 0; i < len(items); i = i + 1 {
        let item: i32 = at(items, i);  // Scoped to loop body
        total = total + item;
    }

    // 'item' not accessible here - proper scoping
    return total;
}
```

---

### Rule 7: Check Return Values

> "Each calling function must check the return value of nonvoid functions, and each called function must check the validity of all parameters."

**Rationale**: Ignoring return values is a frequent source of bugs, especially for error conditions.

**FastC Implementation**: Enforced by type system
- `opt(T)` requires explicit unwrapping
- `res(T, E)` requires handling both success and error
- `discard` keyword for intentionally ignored values

```c
fn parse_number(s: slice(u8)) -> opt(i32) {
    // Returns none() on invalid input
}

fn process() -> i32 {
    let input: arr(u8, 3) = [49, 50, 51];  // "123"

    // VIOLATION: Ignoring optional result
    // let value: i32 = parse_number(slice_from(input));  // Type error!

    // COMPLIANT: Explicit handling
    if let value = parse_number(slice_from(input)) {
        return value;
    }
    return 0;  // Default for parse failure
}

fn log_message(msg: slice(u8)) -> bool {
    // Returns false on failure
}

fn example() {
    let msg: arr(u8, 5) = [72, 101, 108, 108, 111];

    // Explicitly discard if you don't care about result
    discard log_message(slice_from(msg));
}
```

---

### Rule 8: Limited Preprocessor

> "The use of the preprocessor must be limited to file inclusion and simple macro definitions. Token pasting, variable argument lists, and recursive macros are not allowed."

**Rationale**: The C preprocessor can destroy code clarity and confuse static analyzers. Complex macros are notoriously hard to debug.

**FastC Implementation**: Fully satisfied by design
- No preprocessor at all
- Modules replace `#include`
- Constants replace simple macros
- Generics (planned) replace complex macros

```c
// FastC: No preprocessor needed

// Module system replaces #include
use mylib::Vector;
use utils::{min, max};

// Constants replace #define
const MAX_SIZE: usize = 1024;
const PI: f64 = 3.14159265359;

// Functions replace function-like macros
fn square(x: i32) -> i32 {
    return x * x;
}
```

---

### Rule 9: Restricted Pointers

> "No more than one level of dereferencing should be used. Pointer dereference operations may not be hidden in macro definitions. Function pointers are not permitted."

**Rationale**: Pointers are easily misused and make data flow analysis difficult. Multi-level dereferencing compounds the complexity.

**FastC Implementation**:
- Single dereference level enforced
- Safe reference types (`ref`, `mref`) preferred
- Function pointers not in safe code

```c
// VIOLATION: Double dereference
unsafe fn bad_pointers() {
    let x: i32 = 42;
    let p: raw(i32) = addr(x);
    let pp: raw(raw(i32)) = addr(p);
    let value: i32 = deref(deref(pp));  // Error: depth 2 exceeds limit of 1
}

// COMPLIANT: Single dereference
fn good_pointers() {
    let x: i32 = 42;
    let r: ref(i32) = addr(x);
    let value: i32 = deref(r);  // Single level - OK
}

// COMPLIANT: Safe references
fn process(data: ref(i32)) -> i32 {
    return deref(data) * 2;
}
```

---

### Rule 10: Zero Warnings

> "All code must be compiled with all compiler warnings enabled at the most pedantic setting. All code must compile without warnings and pass all static analyzers with zero warnings."

**Rationale**: Modern static analyzers are fast and accurate. There is no excuse for ignoring their output.

**FastC Implementation**:
- `--strict` flag treats warnings as errors
- All P10 violations are errors by default
- Integrates with C compiler warnings for generated code

```bash
# Strict mode - zero tolerance for warnings
fastc compile --strict src/main.fc -o main.c

# Recommended: Enable strict in CI/CD
fastc check --safety-level=critical --strict src/
```

---

## CLI Reference

### Check Command

```bash
# Standard safety (default)
fastc check src/main.fc

# Full Power of 10 compliance
fastc check --safety-level=critical src/main.fc

# Strict mode (warnings as errors)
fastc check --strict src/main.fc

# Relaxed for prototyping
fastc check --safety-level=relaxed src/main.fc
```

### Compile Command

```bash
# Compile with standard safety checks
fastc compile src/main.fc -o main.c

# Compile with full P10 compliance
fastc compile --safety-level=critical src/main.fc -o main.c

# Compile with strict mode
fastc compile --strict src/main.fc -o main.c
```

### List Rules

```bash
# Show enabled rules for a safety level
fastc p10-rules --safety-level=standard
fastc p10-rules --safety-level=critical
```

---

## Configuration

Power of 10 settings can be configured in `fastc.toml`:

```toml
[package]
name = "flight_control"
version = "1.0.0"

[p10]
safety_level = "critical"    # "standard", "critical", or "relaxed"
max_function_lines = 60      # Rule 4 limit
max_pointer_depth = 1        # Rule 9 limit
strict = true                # Rule 10: treat warnings as errors
```

---

## Complete Example

A compliant safety-critical function:

```c
// Flight controller module - Power of 10 compliant

const MAX_ALTITUDE: i32 = 35000;
const MIN_ALTITUDE: i32 = 0;

fn clamp_altitude(current: i32, target: i32) -> i32 {
    // Rule 7: Validate parameters
    if target < MIN_ALTITUDE {
        return MIN_ALTITUDE;
    }
    if target > MAX_ALTITUDE {
        return MAX_ALTITUDE;
    }

    // Rule 2: Bounded loop
    let steps: i32 = 0;
    let max_steps: i32 = 100;
    let altitude: i32 = current;

    while (steps < max_steps && altitude != target) {
        if altitude < target {
            altitude = altitude + 1;
        } else {
            altitude = altitude - 1;
        }
        steps = steps + 1;
    }

    return altitude;
}

fn calculate_descent_rate(
    current_alt: i32,
    target_alt: i32,
    time_remaining: i32
) -> opt(i32) {
    // Rule 7: Return optional for potential failure
    if time_remaining <= 0 {
        return none(i32);
    }

    let delta: i32 = current_alt - target_alt;
    let rate: i32 = delta / time_remaining;

    return some(rate);
}

fn main() -> i32 {
    let current: i32 = 30000;
    let target: i32 = 10000;

    // Rule 7: Handle optional return
    if let rate = calculate_descent_rate(current, target, 20) {
        let safe_alt: i32 = clamp_altitude(current, target);
        return 0;
    }

    return 1;  // Error: invalid parameters
}
```

---

## See Also

- [Safety Guarantees](safety.md) - Memory safety features
- [Certification & AI](certification.md) - Compliance reports for CI/CD and AI agents
- [Unsafe Code](../language/unsafe.md) - When and how to use unsafe
- [C Interoperability](../c-interop/index.md) - FFI safety considerations
