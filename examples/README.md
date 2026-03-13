# FastC Examples

This directory contains example programs demonstrating FastC features and patterns.

## Tutorial Examples (`tutorials/`)

A progressive series for learning FastC basics:

| File | Topic | Concepts |
|------|-------|----------|
| `01_hello_world.fc` | Hello World | `fn main`, `return` |
| `02_variables.fc` | Variables | `let`, types, arithmetic |
| `03_functions.fc` | Functions | parameters, return types, recursion |
| `04_control_flow.fc` | Control Flow | `if/else`, `while`, `for`, `switch` |
| `05_arrays_slices.fc` | Arrays & Slices | `arr(T,N)`, `slice(T)`, `at()` |
| `06_pointers.fc` | Pointers | `ref`, `mref`, `raw`, `addr`, `deref` |
| `07_optionals.fc` | Optionals | `opt(T)`, `some`, `none`, `if-let` |
| `08_results.fc` | Results | `res(T,E)`, error handling patterns |
| `09_structs.fc` | Structs | `@repr(C)`, struct literals |
| `10_enums.fc` | Enums | variants, `switch`, exhaustive matching |

## Advanced Examples (`advanced/`)

Real-world patterns and techniques:

| File | Description |
|------|-------------|
| `algorithms.fc` | Common algorithms (search, GCD, power) |
| `bitflags.fc` | Bitwise operations for flags/permissions |
| `constants.fc` | Compile-time constants and utilities |
| `error_handling.fc` | Error handling patterns with `opt(T)` |
| `ffi_libc.fc` | FFI bindings to libc functions |
| `nng_echo.fc` | NNG networking library FFI example |
| `recursion.fc` | Recursive algorithms (factorial, fibonacci) |
| `state_machine.fc` | Enum-based state machine pattern |
| `type_patterns.fc` | Type safety patterns (newtypes, wrappers) |
| `unsafe_patterns.fc` | When and how to use `unsafe` blocks |

## Running Examples

```bash
# Type-check an example
cargo run --bin fastc -- check examples/tutorials/01_hello_world.fc

# Compile to C
cargo run --bin fastc -- compile examples/tutorials/01_hello_world.fc -o hello.c

# Compile the generated C and run
clang -std=c11 hello.c -o hello
./hello
```

## FFI Examples

The FFI examples (`ffi_libc.fc`, `nng_echo.fc`) require external libraries:

```bash
# Compile with libc (implicit on most systems)
clang -std=c11 output.c -o program

# Compile with nng library
clang -std=c11 output.c -o program -lnng
```

## Build Integration

See `examples/build-integration/` for integrating FastC with Make, CMake, and Meson.
