# Installation

FastC is currently distributed as source code. Follow these steps to build and install it.

## Prerequisites

Ensure you have the following installed:

- **Rust** 1.70 or later
- **Git**
- **A C compiler** (gcc, clang, or MSVC)

### Installing Rust

If you don't have Rust installed:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Verify the installation:

```bash
rustc --version
cargo --version
```

## Building from Source

### 1. Clone the Repository

```bash
git clone https://github.com/Skelf-Research/fastc.git
cd fastc
```

### 2. Build the Project

```bash
cargo build --release
```

This creates the `fastc` binary in `target/release/`.

### 3. Install (Optional)

To install system-wide:

```bash
cargo install --path crates/fastc
```

Or manually copy the binary:

```bash
# Linux/macOS
sudo cp target/release/fastc /usr/local/bin/

# Or add to your PATH
export PATH="$PATH:$(pwd)/target/release"
```

## Verify Installation

Check that FastC is working:

```bash
fastc --help
```

You should see:

```
FastC transpiler - compile FastC to C11

Usage: fastc <COMMAND>

Commands:
  compile  Compile a FastC source file to C
  check    Type-check a FastC source file without emitting C
  fmt      Format a FastC source file
  new      Create a new FastC project
  init     Initialize a FastC project in the current directory
  build    Build the project using fastc.toml configuration
  run      Build, compile, and run the project
  fetch    Fetch project dependencies without building
  help     Print this message or the help of the given subcommand(s)
```

## Runtime Header

FastC programs need the runtime header `fastc_runtime.h`. The compiler automatically finds it when:

1. It's in the `runtime/` directory relative to the executable
2. The `FASTC_RUNTIME` environment variable points to it
3. It's installed in `/usr/local/share/fastc/runtime/`

For manual compilation, use:

```bash
gcc -I/path/to/fastc/runtime your_file.c -o your_program
```

## Troubleshooting

### "command not found: fastc"

Ensure the binary is in your PATH:

```bash
# Check where it was installed
which fastc

# Or use the full path
/path/to/fastc/target/release/fastc --help
```

### Build Errors

Make sure you have the latest Rust:

```bash
rustup update
```

### C Compiler Not Found

The `fastc run` command needs a C compiler. Install one:

```bash
# Ubuntu/Debian
sudo apt install gcc

# macOS
xcode-select --install

# Fedora
sudo dnf install gcc
```

## Next Steps

Now that FastC is installed, proceed to the [Quick Start Guide](quickstart.md) to create your first project.
