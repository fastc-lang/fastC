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
