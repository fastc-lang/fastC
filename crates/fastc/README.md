# FastC

A safe C-like language that compiles to C11.

FastC is a source-to-source transpiler that emits standard C11 code. It removes common C footguns while keeping the C toolchain, ABI, and performance model intact.

## Features

- **Safe by default** - Null safety with `opt(T)`, bounds-checked arrays, explicit unsafe blocks
- **C11 output** - Works with any C compiler (gcc, clang, MSVC)
- **Zero runtime overhead** - No garbage collector, no hidden allocations
- **C interop** - Call C functions, expose APIs to C, use existing libraries

## Installation

```bash
# Build from source
cargo install --path .

# Or build the workspace
cargo build --release
```

## Usage

```bash
# Compile FastC to C
fastc compile input.fc -o output.c

# Generate header file
fastc compile input.fc -o output.c --emit-header

# Build and compile to executable
fastc build --cc

# Build and run
fastc run
```

## Example

```c
// hello.fc
fn main() -> i32 {
    return 0;
}
```

```bash
fastc compile hello.fc -o hello.c
cc -I /path/to/fastc/runtime hello.c -o hello
./hello
```

## Documentation

- [User Guide](https://docs.skelfresearch.com/fastc)
- [Language Reference](https://docs.skelfresearch.com/fastc/language/)
- [C Interop](https://docs.skelfresearch.com/fastc/c-interop/)

## License

This project is licensed under the [MIT License](../../LICENSE).
