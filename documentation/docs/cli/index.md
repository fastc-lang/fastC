# CLI Reference

The `fastc` command-line tool provides everything you need to develop FastC programs.

## Commands Overview

| Command | Description |
|---------|-------------|
| `compile` | Transpile FastC to C |
| `check` | Type-check without generating code |
| `fmt` | Format source code |
| `new` | Create a new project |
| `init` | Initialize project in existing directory |
| `build` | Build project from fastc.toml |
| `run` | Build, compile, and run |
| `fetch` | Fetch dependencies |

## Quick Reference

```bash
# Type-check a file
fastc check src/main.fc

# Compile to C (stdout)
fastc compile src/main.fc

# Compile to file
fastc compile src/main.fc -o main.c

# Format code
fastc fmt src/main.fc

# Create new project
fastc new my_project

# Build and run
fastc run
```

## Getting Help

```bash
# General help
fastc --help

# Command-specific help
fastc compile --help
fastc build --help
fastc run --help
```

## Topics

- [Compile](compile.md) - Transpiling to C
- [Build & Run](build-run.md) - Building and running projects
- [Project Management](project.md) - Creating and configuring projects
