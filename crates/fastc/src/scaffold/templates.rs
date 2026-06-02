//! Template strings for project scaffolding

use super::{BuildTemplate, ProjectType};

/// Generate fastc.toml manifest content
pub fn manifest(name: &str, project_type: ProjectType) -> String {
    let type_str = match project_type {
        ProjectType::Binary => "binary",
        ProjectType::Library => "library",
        ProjectType::FfiWrapper => "ffi-wrapper",
    };

    format!(
        r#"[package]
name = "{name}"
version = "0.1.0"
type = "{type_str}"

[build]
# Include directories for C compilation
# include_dirs = ["include"]

# Libraries to link
# link_libs = []

[dependencies]
# Git dependencies with version pinning
# example = {{ git = "https://github.com/user/example", tag = "v1.0.0" }}
# local_dep = {{ path = "../local_dep" }}
"#
    )
}

/// Generate main.fc for binary projects
pub fn main_fc(name: &str) -> String {
    format!(
        r#"// {name} - A FastC application

fn main() -> i32 {{
    // Your code here
    return 0;
}}
"#
    )
}

/// Generate lib.fc for library projects
pub fn lib_fc(name: &str) -> String {
    format!(
        r#"// {name} - A FastC library

// Example function
fn add(a: i32, b: i32) -> i32 {{
    return (a + b);
}}

// Example constant
const VERSION: i32 = 1;
"#
    )
}

/// Generate lib.fc for FFI wrapper projects
pub fn ffi_lib_fc(name: &str) -> String {
    format!(
        r#"// {name} - A FastC FFI wrapper library

// Declare external C functions
extern "C" {{
    // Example: wrap a C library function
    // unsafe fn external_func(arg: i32) -> i32;
}}

// Safe wrapper around unsafe FFI
fn wrapped_function(x: i32) -> i32 {{
    // unsafe {{
    //     return external_func(x);
    // }}
    return x;
}}

// Export functions for C callers
fn {name}_init() -> i32 {{
    return 0;
}}

fn {name}_cleanup() -> void {{
    // Cleanup code here
}}
"#,
        name = name.replace('-', "_")
    )
}

/// Generate Makefile
pub fn makefile(name: &str, project_type: ProjectType) -> String {
    let src_file = match project_type {
        ProjectType::Binary => "main",
        ProjectType::Library | ProjectType::FfiWrapper => "lib",
    };

    let target = match project_type {
        ProjectType::Binary => name.to_string(),
        ProjectType::Library | ProjectType::FfiWrapper => {
            format!("lib{}.a", name.replace('-', "_"))
        }
    };

    let emit_header = if project_type == ProjectType::FfiWrapper {
        " --emit-header"
    } else {
        ""
    };

    let build_rule = match project_type {
        ProjectType::Binary => r#"$(TARGET): build/$(SRC_NAME).o
	$(CC) $(CFLAGS) -o $@ $<"#
            .to_string(),
        ProjectType::Library | ProjectType::FfiWrapper => r#"$(TARGET): build/$(SRC_NAME).o
	$(AR) rcs $@ $<"#
            .to_string(),
    };

    format!(
        r#"# {name} - FastC project Makefile

# Configuration
FASTC ?= fastc
CC ?= clang
CFLAGS ?= -std=c11 -Wall -Wextra -O2
AR ?= ar

# Project
SRC_NAME = {src_file}
TARGET = {target}

# Directories
SRC_DIR = src
BUILD_DIR = build

# Rules
.PHONY: all clean check fmt

all: $(TARGET)

{build_rule}

build/$(SRC_NAME).o: build/$(SRC_NAME).c
	$(CC) $(CFLAGS) -c -o $@ $<

build/$(SRC_NAME).c: $(SRC_DIR)/$(SRC_NAME).fc | $(BUILD_DIR)
	$(FASTC) compile $<{emit_header} -o $@

$(BUILD_DIR):
	mkdir -p $(BUILD_DIR)

check:
	$(FASTC) check $(SRC_DIR)/$(SRC_NAME).fc

fmt:
	$(FASTC) fmt $(SRC_DIR)/$(SRC_NAME).fc

clean:
	rm -rf $(BUILD_DIR) $(TARGET)
"#
    )
}

/// Generate CMakeLists.txt
pub fn cmakelists(name: &str, project_type: ProjectType) -> String {
    let (src_file, target_type) = match project_type {
        ProjectType::Binary => ("main", "add_executable"),
        ProjectType::Library => ("lib", "add_library"),
        ProjectType::FfiWrapper => ("lib", "add_library"),
    };

    let emit_header = if project_type == ProjectType::FfiWrapper {
        " --emit-header"
    } else {
        ""
    };

    format!(
        r#"# {name} - FastC project CMakeLists.txt
cmake_minimum_required(VERSION 3.16)
project({name} C)

set(CMAKE_C_STANDARD 11)
set(CMAKE_C_STANDARD_REQUIRED ON)

# Find FastC compiler
find_program(FASTC fastc REQUIRED)

# Source files
set(FC_SOURCE "${{CMAKE_SOURCE_DIR}}/src/{src_file}.fc")
set(C_OUTPUT "${{CMAKE_BINARY_DIR}}/{src_file}.c")

# Custom command to compile FastC to C
add_custom_command(
    OUTPUT ${{C_OUTPUT}}
    COMMAND ${{FASTC}} compile ${{FC_SOURCE}}{emit_header} -o ${{C_OUTPUT}}
    DEPENDS ${{FC_SOURCE}}
    COMMENT "Compiling FastC to C"
)

# Build target
{target_type}({name} ${{C_OUTPUT}})

# Custom target for type-checking
add_custom_target(check
    COMMAND ${{FASTC}} check ${{FC_SOURCE}}
    COMMENT "Type-checking FastC source"
)

# Custom target for formatting
add_custom_target(fmt
    COMMAND ${{FASTC}} fmt ${{FC_SOURCE}}
    COMMENT "Formatting FastC source"
)
"#
    )
}

/// Generate meson.build
pub fn meson_build(name: &str, project_type: ProjectType) -> String {
    let (src_file, target_func) = match project_type {
        ProjectType::Binary => ("main", "executable"),
        ProjectType::Library => ("lib", "static_library"),
        ProjectType::FfiWrapper => ("lib", "static_library"),
    };

    let emit_header = if project_type == ProjectType::FfiWrapper {
        ", '--emit-header'"
    } else {
        ""
    };

    format!(
        r#"# {name} - FastC project meson.build
project('{name}', 'c',
  version: '0.1.0',
  default_options: ['c_std=c11', 'warning_level=2']
)

# Find FastC compiler
fastc = find_program('fastc', required: true)

# Generate C from FastC
fc_source = files('src/{src_file}.fc')
c_output = custom_target('{src_file}_c',
  input: fc_source,
  output: '{src_file}.c',
  command: [fastc, 'compile', '@INPUT@'{emit_header}, '-o', '@OUTPUT@']
)

# Build target
{name}_target = {target_func}('{name}', c_output)

# Type-check target
run_target('check',
  command: [fastc, 'check', fc_source]
)

# Format target
run_target('fmt',
  command: [fastc, 'fmt', fc_source]
)
"#
    )
}

/// Generate .gitignore
pub fn gitignore() -> String {
    r#"# Build outputs
/build/
*.o
*.a
*.so
*.dylib

# Generated C files
*.c
*.h
!src/*.h

# Editor files
*.swp
*.swo
*~
.vscode/
.idea/

# OS files
.DS_Store
Thumbs.db
"#
    .to_string()
}

/// Generate README.md
pub fn readme(name: &str, project_type: ProjectType, build_template: BuildTemplate) -> String {
    let type_desc = match project_type {
        ProjectType::Binary => "A FastC application",
        ProjectType::Library => "A FastC library",
        ProjectType::FfiWrapper => "A FastC FFI wrapper library",
    };

    let build_instructions = match build_template {
        BuildTemplate::Make => {
            r#"```bash
# Build the project
make

# Type-check without building
make check

# Format source code
make fmt

# Clean build artifacts
make clean
```"#
        }
        BuildTemplate::CMake => {
            r#"```bash
# Configure
mkdir build && cd build
cmake ..

# Build
make

# Type-check
make check

# Format
make fmt
```"#
        }
        BuildTemplate::Meson => {
            r#"```bash
# Configure
meson setup build

# Build
meson compile -C build

# Type-check
meson compile -C build check

# Format
meson compile -C build fmt
```"#
        }
    };

    format!(
        r#"# {name}

{type_desc}.

## Building

{build_instructions}

## Project Structure

```
{name}/
├── fastc.toml      # Project manifest
├── src/
│   └── *.fc        # FastC source files
└── build/          # Build outputs (generated)
```

## License

MIT
"#
    )
}

/// Generate AGENTS.md — the agent-facing entry point that ships with
/// every new fastC project (stage 1.6 / 2.1 DoD). Tells an LLM
/// (a) what language this is, (b) which compiler entry points
/// produce structured artifacts, (c) the capability and contract
/// surfaces it should query before generating code.
pub fn agents_md(name: &str, project_type: ProjectType) -> String {
    let type_desc = match project_type {
        ProjectType::Binary => "a binary fastC application",
        ProjectType::Library => "a fastC library",
        ProjectType::FfiWrapper => "a fastC FFI wrapper around an existing C library",
    };

    format!(
        r#"# AGENTS.md — {name}

Self-describing entry point for AI coding agents (Claude Code, Cursor,
Codex, …) working in this repository. fastC is **C-like, memory-safe
without a GC, capability-typed I/O, contract-annotated, no executable
build scripts.** This project is {type_desc}.

## TL;DR for agents

- Source files live in `src/*.fc`. Build with `fastc build`. Run with
  `fastc run`.
- Use `fastc explain src/<file>.fc` to get a JSON document of every
  function in that file: name, params, return type, declared
  capabilities, `@requires` / `@ensures` contracts, doc comments.
  Prefer this over reading the source.
- Use `fastc compile <file> --caps-output=- --discharge-output=- -o /dev/null`
  to get the **capability surface** and **contract discharge report**
  of any fastC file. The caps.json lists which `Cap*` tokens each fn
  demands; the discharge.json shows which contracts were proven
  statically vs left as runtime asserts.
- Power-of-10 safety rules are on by default. Use
  `fastc check src/<file>.fc` to verify; `--safety-level=critical`
  enforces the full set (no recursion, no heap, bounded loops, etc.).
- For agent-native diagnostics: `fastc-mcp` runs over stdio
  JSON-RPC; the binary lives next to `fastc` after `cargo install`.
  See `docs/mcp.md` in the fastc repo.

## The wedge — what makes fastC different from C / Rust / Zig / Go

1. **Capability-typed I/O.** A function that reads files takes
   `c: ref(CapFsRead)` as a parameter. A function with no `Cap*`
   parameters structurally cannot reach the filesystem, the network,
   the clock, env vars, or anything else — the type system makes
   ambient authority impossible.
2. **No executable build scripts.** `fastc.toml` is closed-schema
   TOML enforced by `#[serde(deny_unknown_fields)]`. There is no
   place to put code that runs at install / build time — supply
   chain attacks (faster_log, async_println, CVE-2026-28353) don't
   apply.
3. **Compile-time contracts.** `@requires(x > 0)` and
   `@ensures(result >= 0)` are first-class on every function. A
   three-tier discharge pipeline (syntactic → SMT → runtime) proves
   what it can; the rest stays as runtime traps.
4. **Portable C11 output.** fastC compiles to readable C. gdb,
   valgrind, perf, ASan, every C compiler optimisation all work.
   Cross-compile via `fastc build --target=aarch64-linux-musl` etc.

## What to do BEFORE writing code in this repo

1. Read `src/main.fc` (or `src/lib.fc`) to see the existing surface.
2. Run `fastc explain src/main.fc` for the structured signature.
3. If you're adding I/O, route it through a `Cap*` token threaded
   from `main`. Never call `caps::init()` outside `main` — the
   compiler will reject it.
4. If you're adding a new contract, add the smallest tier-1
   tautology if you can — it gets discharged for free.

## Tooling shortcuts

- `fastc compile <file>.fc -o out.c` — emit C without linking.
- `fastc compile <file>.fc -o out.c --prove --discharge-output=-`
  — emit C and print the contract discharge report.
- `fastc target list` — see the cross-compile target matrix.
- `fastc check --safety-level=critical <file>.fc` — full P10 check.
- `fastc bench` — run the compile-time budget gate.

## What NOT to do

- Don't add a `build.rs` equivalent. It doesn't exist on purpose.
- Don't fabricate a `Cap*` struct (`CapFsRead {{}}`) outside `mod caps`
  — the compiler will reject it.
- Don't disable Power of 10 rules to silence a warning. Fix the
  underlying issue or use `--safety-level=relaxed` only as a temporary
  signal that the code needs to be revisited.
- Don't pin a dependency without recording its content sha256 in
  `fastc.lock`. Run `fastc lock` after adding a new dep.

## Reading order if you're new to this project

1. `fastc.toml` — what the project declares about itself.
2. `src/main.fc` (or `src/lib.fc`) — entry point.
3. `fastc explain src/<file>.fc` — JSON signature surface for every
   function. **Use this instead of reading bodies when you're
   collecting context.**
4. This file (`AGENTS.md`).
"#
    )
}
