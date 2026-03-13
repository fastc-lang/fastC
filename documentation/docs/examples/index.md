# Examples

Learn FastC through practical examples, from basic syntax to advanced patterns.

## Example Categories

### Tutorials

Step-by-step tutorials covering fundamental concepts:

| Tutorial | Topic |
|----------|-------|
| 01 | Hello World - Entry point and exit codes |
| 02 | Variables - Types and arithmetic |
| 03 | Functions - Parameters and return values |
| 04 | Control Flow - if/else, loops, switch |
| 05 | Arrays & Slices - Fixed-size and dynamic arrays |
| 06 | Pointers - References and raw pointers |
| 07 | Optionals - opt(T), some, none, if-let |
| 08 | Results - res(T,E) error handling |
| 09 | Structs - User-defined types |
| 10 | Enums - Enumerated types |

### Advanced Examples

Real-world patterns and techniques:

| Example | Description |
|---------|-------------|
| algorithms | Sorting and searching algorithms |
| bitflags | Bitwise operations and flags |
| constants | Compile-time constants |
| error_handling | Error handling patterns |
| ffi_libc | Calling libc functions |
| nng_echo | Network programming with NNG |
| recursion | Recursive algorithms |
| state_machine | Enum-based state machines |
| type_patterns | Advanced type usage |
| unsafe_patterns | Safe wrappers for unsafe code |

## Running Examples

### From Source Directory

```bash
# Compile to C
fastc compile examples/tutorials/01_hello_world.fc -o hello.c

# Compile and run
fastc compile examples/tutorials/01_hello_world.fc -o hello.c
cc -I runtime hello.c -o hello
./hello
```

### With fastc run

```bash
# Build and run in one step
cd examples/tutorials
fastc compile 01_hello_world.fc -o /tmp/hello.c
cc /tmp/hello.c -o /tmp/hello && /tmp/hello
```

## Example Structure

Each example includes:

1. **Comment header** - Explains the concept
2. **Key concepts** - Lists features demonstrated
3. **Working code** - Complete, compilable program
4. **Main function** - Entry point that exercises the code

## Quick Start Example

Here's the simplest FastC program:

```c
// Every program needs main() returning i32
fn main() -> i32 {
    return 0;  // 0 = success
}
```

## Next Steps

- [Tutorials](tutorials.md) - Work through each tutorial
- [Advanced Examples](advanced.md) - Explore real-world patterns
- [Language Guide](../language/index.md) - Comprehensive reference

