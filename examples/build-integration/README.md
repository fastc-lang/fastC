# Build Integration Examples

This directory contains examples of how to integrate FastC into common C build systems.

## Prerequisites

- FastC compiler (`fastc`) installed and in your PATH
- A C11-compatible C compiler (gcc, clang, etc.)

## Make

```bash
cd make
make        # Build the project
make check  # Type-check all .fc files
make fmt    # Format all .fc files
make clean  # Remove generated files
```

## CMake

```bash
cd cmake
mkdir build && cd build
cmake ..
make        # Build the project
make check  # Type-check all .fc files
```

## Meson

```bash
cd meson
meson setup build
cd build
ninja       # Build the project
ninja check # Type-check all .fc files
ninja fmt   # Format all .fc files
```

## How It Works

All build systems follow the same pattern:

1. Find `.fc` source files
2. Run `fastc compile input.fc -o output.c` to transpile each file
3. Compile the generated `.c` files with a standard C compiler
4. Link the object files into the final executable

The generated C code is standard C11 and should work with any compliant compiler.
