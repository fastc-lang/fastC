# FastC

[![Build Status](https://github.com/Skelf-Research/fastc/workflows/CI/badge.svg)](https://github.com/Skelf-Research/fastc/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.85%2B-orange.svg)](https://www.rust-lang.org/)
[![Documentation](https://img.shields.io/badge/docs-online-green.svg)](https://docs.skelfresearch.com/fastc)

**C, but safe and agent-friendly.**

FastC is a modern C-like language designed for the age of AI-assisted development. It compiles to readable C11, eliminates undefined behavior in safe code, and provides a predictable syntax that both humans and AI agents can reason about confidently.

```c
fn main() -> i32 {
    let numbers: arr(i32, 5) = [1, 2, 3, 4, 5];
    let sum: i32 = 0;

    for let i: i32 = 0; i < 5; i = i + 1 {
        sum = sum + at(numbers, i);  // Bounds-checked
    }

    return sum;
}
```

## Why FastC?

### Agent-Friendly by Design

AI coding assistants struggle with C's ambiguous grammar, implicit conversions, and undefined behavior. FastC fixes this:

| C Problem | FastC Solution |
|-----------|----------------|
| Ambiguous declarations | Explicit `let name: type` syntax |
| Implicit type coercion | All conversions require `cast()` |
| Null pointer chaos | `opt(T)` type with mandatory checks |
| Buffer overflows | Bounds-checked `at()` for array access |
| Hidden evaluation order | Guaranteed left-to-right evaluation |
| Scattered unsafe code | Explicit `unsafe` blocks |

When an AI agent writes FastC, it knows exactly what the code will do. No surprises.

### Zero-Cost C Interop

FastC compiles to clean, readable C11. Your existing toolchain just works:

```bash
# Compile FastC to C
fastc compile app.fc -o app.c --emit-header

# Use any C compiler
gcc -O2 app.c -o app
clang -O3 app.c -o app
```

Call any C library. Expose APIs to C code. Debug with gdb, profile with perf, sanitize with ASan. Everything you know about C still applies.

### Safe by Default

Safe code cannot cause undefined behavior:

```c
fn safe_divide(a: i32, b: i32) -> opt(i32) {
    if b == 0 {
        return none(i32);
    }
    return some(a / b);
}

fn process(data: slice(i32)) -> i32 {
    // Bounds-checked access - no buffer overflows
    return at(data, 0);
}
```

Need raw performance? Opt into `unsafe` explicitly:

```c
unsafe fn fast_copy(dst: rawm(u8), src: raw(u8), n: usize) {
    // You're responsible now
    extern "C" { fn memcpy(d: rawm(u8), s: raw(u8), n: usize) -> rawm(u8); }
    discard memcpy(dst, src, n);
}
```

## Quick Start

### Install

```bash
# From source
git clone https://github.com/Skelf-Research/fastc.git
cd fastc
cargo install --path crates/fastc

# Verify installation
fastc --version
```

### Hello World

```c
// hello.fc
fn main() -> i32 {
    return 0;
}
```

```bash
fastc compile hello.fc -o hello.c
cc hello.c -o hello
./hello && echo "Success!"
```

### Create a Project

```bash
fastc new my_project
cd my_project
fastc build --cc
fastc run
```

## Features

- **Explicit Types** - `let x: i32 = 42` not `int x = 42`
- **Safe Pointers** - `ref(T)`, `mref(T)` for safe references; `raw(T)`, `rawm(T)` for unsafe
- **Optionals** - `opt(T)` with `some()`, `none()`, and `if let` unwrapping
- **Results** - `res(T, E)` for error handling without exceptions
- **Slices** - `slice(T)` with bounds checking via `at()`
- **Fixed Arrays** - `arr(T, N)` with compile-time size
- **C FFI** - `extern "C"` blocks for calling C functions
- **Header Generation** - `--emit-header` produces C headers for your API

## Documentation

- **[Getting Started](https://docs.skelfresearch.com/fastc/getting-started/)** - Installation and first project
- **[Language Guide](https://docs.skelfresearch.com/fastc/language/)** - Complete language reference
- **[C Interop](https://docs.skelfresearch.com/fastc/c-interop/)** - Calling C and exposing APIs
- **[CLI Reference](https://docs.skelfresearch.com/fastc/cli/)** - All commands and options

## Editor Support

FastC includes an LSP server for IDE integration:

```bash
cargo install --path crates/fastc-lsp
```

- **VS Code** - Install the FastC extension
- **Neovim** - Configure with nvim-lspconfig
- **Helix** - Add to languages.toml

See [Editor Setup](https://docs.skelfresearch.com/fastc/getting-started/editor-setup/) for details.

## Project Structure

```
fastc/
├── crates/
│   ├── fastc/          # Compiler and CLI
│   └── fastc-lsp/      # Language server
├── runtime/            # C runtime header
├── examples/           # Example programs
│   ├── tutorials/      # Learning examples (01-10)
│   └── advanced/       # Real-world patterns
└── documentation/      # MkDocs source
```

## Design Principles

1. **No ambiguity** - Every construct has exactly one meaning
2. **Explicit over implicit** - Types, casts, and unsafe are always visible
3. **C compatibility** - Output is standard C11, ABI-compatible
4. **Predictable codegen** - Same input always produces same output
5. **Minimal runtime** - Just a small header, no hidden allocations

## Contributing

We welcome contributions! Please:

1. Check existing issues before creating new ones
2. Keep proposals concrete and testable
3. Include tests for new features
4. Run `cargo test` before submitting PRs

## License

This project is licensed under the [MIT License](LICENSE).

---

<p align="center">
  <b>FastC</b> — Making C safe for humans and agents alike.<br>
  <a href="https://github.com/Skelf-Research/fastc">GitHub</a> ·
  <a href="https://docs.skelfresearch.com/fastc">Documentation</a> ·
  <a href="https://github.com/Skelf-Research/fastc/issues">Issues</a>
</p>
