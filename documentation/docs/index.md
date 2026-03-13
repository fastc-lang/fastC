# FastC

**A safe, C-like language that compiles to C11**

FastC is a transpiled language designed for systems programming with a focus on safety, C interoperability, and familiar syntax. Write code that looks like C but with modern safety features, then compile it to portable C11.

## Key Features

- **Familiar Syntax** - If you know C, you already know most of FastC
- **Memory Safety** - Bounds checking, null safety, and overflow detection
- **Zero Runtime** - Compiles to standard C11 with no runtime dependencies
- **C Interop** - Seamless FFI with existing C libraries
- **Modern Types** - Optionals, results, slices, and more

## Quick Example

```c
// hello.fc
fn main() -> i32 {
    let message: slice(u8) = c"Hello, FastC!";
    return 0;
}
```

Compile and run:

```bash
fastc new hello
cd hello
fastc run
```

## Why FastC?

| Feature | C | FastC |
|---------|---|-------|
| Null pointer dereference | Undefined behavior | Compile-time prevention |
| Array bounds | Unchecked | Runtime checked (safe mode) |
| Integer overflow | Undefined (signed) | Runtime checked (safe mode) |
| Optional values | Manual NULL checks | `opt(T)` with `if-let` |
| Error handling | Return codes | `res(T,E)` result types |

## Getting Started

1. [Install FastC](getting-started/installation.md) from source
2. Follow the [Quick Start Guide](getting-started/quickstart.md)
3. Explore the [Language Guide](language/index.md)

## Project Status

FastC is under active development. Current features:

- Complete type system with primitives, structs, enums
- Optional and result types for safe error handling
- Module system for multi-file projects
- Full C FFI support
- CLI tools: compile, check, format, build, run

## Links

- [GitHub Repository](https://github.com/Skelf-Research/fastc)
- [Issue Tracker](https://github.com/Skelf-Research/fastc/issues)
- [Skelf Research](https://skelfresearch.com)

## License

FastC is open source software licensed under the MIT License.
