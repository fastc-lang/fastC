# Build and Run Commands

The `build` and `run` commands provide a complete workflow for compiling and executing FastC programs.

## Build Command

Build a project using its `fastc.toml` configuration.

### Usage

```bash
fastc build [OPTIONS]
```

### Options

| Option | Description |
|--------|-------------|
| `--release` | Build with optimizations |
| `-o, --output <DIR>` | Output directory (default: `build`) |
| `--cc` | Also compile C to executable |
| `--compiler <CC>` | C compiler to use (default: `cc`) |
| `--cflags <FLAGS>` | Additional C compiler flags |
| `-h, --help` | Print help |

### Examples

```bash
# Basic build (generates C only)
fastc build

# Build with C compilation
fastc build --cc

# Release build
fastc build --cc --release

# Use clang
fastc build --cc --compiler clang

# Add warnings
fastc build --cc --cflags "-Wall -Wextra"
```

### Build Output

```
No dependencies to fetch.
Compiling: /path/to/project/src/main.fc
  Wrote: build/main.c
  Wrote: build/main.h
FastC compilation complete.
```

With `--cc`:

```
Compiling C code with cc...
  cc build/main.c -o build/main -I /path/to/runtime -g -O0 -lm
  Wrote: build/main
C compilation complete.
```

## Run Command

Build, compile, and run the project in one step.

### Usage

```bash
fastc run [OPTIONS] [-- <ARGS>...]
```

### Options

| Option | Description |
|--------|-------------|
| `--release` | Build with optimizations |
| `--compiler <CC>` | C compiler to use (default: `cc`) |
| `--cflags <FLAGS>` | Additional C compiler flags |
| `<ARGS>...` | Arguments passed to the program |
| `-h, --help` | Print help |

### Examples

```bash
# Build and run
fastc run

# Release mode
fastc run --release

# Pass arguments to program
fastc run -- arg1 arg2 arg3

# Use clang with warnings
fastc run --compiler clang --cflags "-Wall"
```

### Run Output

```
No dependencies to fetch.
Compiling: /path/to/project/src/main.fc
  Wrote: build/main.c
  Wrote: build/main.h
FastC compilation complete.
Compiling C code with cc...
  Wrote: build/main
C compilation complete.
Running: build/main
---
Hello, FastC!
---
Program exited with code: 0
```

## Compiler Flags

### Debug Mode (Default)

- `-g` - Debug symbols
- `-O0` - No optimization

### Release Mode (`--release`)

- `-O2` - Optimization level 2
- `-DNDEBUG` - Disable assertions

### Common Custom Flags

```bash
# Enable all warnings
fastc run --cflags "-Wall -Wextra -Wpedantic"

# Treat warnings as errors
fastc run --cflags "-Werror"

# Link additional libraries
fastc run --cflags "-lpthread -lssl"

# Specify C standard
fastc run --cflags "-std=c11"
```

## Fetch Command

Fetch dependencies without building:

```bash
fastc fetch
```

This downloads all dependencies specified in `fastc.toml` and updates `fastc.lock`.

### Output

```
Fetching dependency: mylib
  Fetched to: /home/user/.cache/fastc/deps/mylib/abc123
Updated fastc.lock
Dependencies fetched successfully.
```

## Build Directory Structure

After `fastc build`:

```
project/
├── fastc.toml
├── fastc.lock          # Created after fetch
├── src/
│   └── main.fc
└── build/
    ├── main.c          # Generated C code
    ├── main.h          # Generated header
    └── main            # Executable (with --cc)
```

## Environment Variables

| Variable | Description |
|----------|-------------|
| `FASTC_RUNTIME` | Path to runtime headers |
| `CC` | Default C compiler (used if --compiler not specified) |

### Example

```bash
export FASTC_RUNTIME=/opt/fastc/runtime
export CC=clang
fastc run
```

## Troubleshooting

### "Runtime header not found"

Set `FASTC_RUNTIME` or ensure the runtime is in a standard location:

```bash
export FASTC_RUNTIME=/path/to/fastc/runtime
```

### "cc: command not found"

Install a C compiler:

```bash
# Ubuntu/Debian
sudo apt install gcc

# macOS
xcode-select --install

# Or specify a different compiler
fastc run --compiler /usr/bin/gcc
```

### "undefined reference to fc_trap"

The runtime header isn't being found. Check:

1. `FASTC_RUNTIME` is set correctly
2. The path contains `fastc_runtime.h`

## See Also

- [Project Management](project.md) - Project configuration
- [Compile](compile.md) - Manual compilation
