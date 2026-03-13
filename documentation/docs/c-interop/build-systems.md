# Build System Integration

FastC integrates with standard C build systems.

## General Workflow

1. Run `fastc compile` to generate `.c` and `.h` files
2. Compile generated C with your build system
3. Link with other C code as needed

## GNU Make

### Basic Makefile

```makefile
# Compiler settings
CC ?= gcc
CFLAGS ?= -Wall -O2
FASTC ?= fastc

# Directories
SRC_DIR = src
BUILD_DIR = build

# Find all FastC sources
FC_SOURCES = $(wildcard $(SRC_DIR)/*.fc)
C_SOURCES = $(patsubst $(SRC_DIR)/%.fc,$(BUILD_DIR)/%.c,$(FC_SOURCES))

# Runtime include path
FASTC_RUNTIME ?= /path/to/fastc/runtime

# Default target
all: $(BUILD_DIR)/main

# Generate C from FastC
$(BUILD_DIR)/%.c: $(SRC_DIR)/%.fc | $(BUILD_DIR)
	$(FASTC) compile $< -o $@ --emit-header

# Compile C to executable
$(BUILD_DIR)/main: $(BUILD_DIR)/main.c
	$(CC) $(CFLAGS) -I$(FASTC_RUNTIME) $< -o $@

# Create build directory
$(BUILD_DIR):
	mkdir -p $@

# Clean
clean:
	rm -rf $(BUILD_DIR)

.PHONY: all clean
```

### Library Makefile

```makefile
CC ?= gcc
AR ?= ar
CFLAGS ?= -Wall -O2 -fPIC
FASTC ?= fastc

BUILD_DIR = build
FASTC_RUNTIME ?= /path/to/fastc/runtime

all: $(BUILD_DIR)/libmylib.a $(BUILD_DIR)/libmylib.so

$(BUILD_DIR)/lib.c: src/lib.fc | $(BUILD_DIR)
	$(FASTC) compile $< -o $@ --emit-header

$(BUILD_DIR)/lib.o: $(BUILD_DIR)/lib.c
	$(CC) $(CFLAGS) -I$(FASTC_RUNTIME) -c $< -o $@

# Static library
$(BUILD_DIR)/libmylib.a: $(BUILD_DIR)/lib.o
	$(AR) rcs $@ $<

# Shared library
$(BUILD_DIR)/libmylib.so: $(BUILD_DIR)/lib.o
	$(CC) -shared $< -o $@

$(BUILD_DIR):
	mkdir -p $@

clean:
	rm -rf $(BUILD_DIR)

.PHONY: all clean
```

## CMake

### Basic CMakeLists.txt

```cmake
cmake_minimum_required(VERSION 3.16)
project(my_fastc_project C)

set(CMAKE_C_STANDARD 11)

# Find fastc
find_program(FASTC fastc REQUIRED)

# Runtime include path
set(FASTC_RUNTIME "$ENV{FASTC_RUNTIME}" CACHE PATH "FastC runtime directory")

# Custom command to compile FastC to C
function(add_fastc_source TARGET SOURCE)
    get_filename_component(BASENAME ${SOURCE} NAME_WE)
    set(OUTPUT_C "${CMAKE_CURRENT_BINARY_DIR}/${BASENAME}.c")
    set(OUTPUT_H "${CMAKE_CURRENT_BINARY_DIR}/${BASENAME}.h")

    add_custom_command(
        OUTPUT ${OUTPUT_C} ${OUTPUT_H}
        COMMAND ${FASTC} compile ${CMAKE_CURRENT_SOURCE_DIR}/${SOURCE}
                -o ${OUTPUT_C} --emit-header
        DEPENDS ${CMAKE_CURRENT_SOURCE_DIR}/${SOURCE}
        COMMENT "Compiling ${SOURCE} with fastc"
    )

    target_sources(${TARGET} PRIVATE ${OUTPUT_C})
    target_include_directories(${TARGET} PRIVATE ${CMAKE_CURRENT_BINARY_DIR})
endfunction()

# Create executable
add_executable(main "")
add_fastc_source(main src/main.fc)

# Include runtime
target_include_directories(main PRIVATE ${FASTC_RUNTIME})
```

### Library CMakeLists.txt

```cmake
cmake_minimum_required(VERSION 3.16)
project(mylib C)

set(CMAKE_C_STANDARD 11)

find_program(FASTC fastc REQUIRED)
set(FASTC_RUNTIME "$ENV{FASTC_RUNTIME}" CACHE PATH "FastC runtime")

# Generate C sources
set(LIB_C "${CMAKE_CURRENT_BINARY_DIR}/lib.c")
set(LIB_H "${CMAKE_CURRENT_BINARY_DIR}/lib.h")

add_custom_command(
    OUTPUT ${LIB_C} ${LIB_H}
    COMMAND ${FASTC} compile ${CMAKE_CURRENT_SOURCE_DIR}/src/lib.fc
            -o ${LIB_C} --emit-header
    DEPENDS src/lib.fc
)

# Static library
add_library(mylib_static STATIC ${LIB_C})
target_include_directories(mylib_static PUBLIC
    ${CMAKE_CURRENT_BINARY_DIR}
    ${FASTC_RUNTIME}
)

# Shared library
add_library(mylib_shared SHARED ${LIB_C})
target_include_directories(mylib_shared PUBLIC
    ${CMAKE_CURRENT_BINARY_DIR}
    ${FASTC_RUNTIME}
)
set_target_properties(mylib_shared PROPERTIES OUTPUT_NAME mylib)

# Install
install(TARGETS mylib_static mylib_shared
    ARCHIVE DESTINATION lib
    LIBRARY DESTINATION lib
)
install(FILES ${LIB_H} DESTINATION include)
```

## Meson

### meson.build

```meson
project('my_fastc_project', 'c',
    version: '0.1.0',
    default_options: ['c_std=c11'])

# Find fastc
fastc = find_program('fastc')

# Runtime include directory
fastc_runtime = include_directories(
    get_option('fastc_runtime'),
    is_system: true
)

# Custom target to generate C from FastC
main_c = custom_target('main_c',
    input: 'src/main.fc',
    output: ['main.c', 'main.h'],
    command: [fastc, 'compile', '@INPUT@', '-o', '@OUTPUT0@', '--emit-header'],
)

# Build executable
executable('main',
    main_c,
    include_directories: fastc_runtime,
)
```

### meson_options.txt

```meson
option('fastc_runtime',
    type: 'string',
    value: '/usr/local/share/fastc/runtime',
    description: 'Path to FastC runtime headers'
)
```

### Library meson.build

```meson
project('mylib', 'c', version: '0.1.0')

fastc = find_program('fastc')
fastc_runtime = include_directories(get_option('fastc_runtime'))

lib_c = custom_target('lib_c',
    input: 'src/lib.fc',
    output: ['lib.c', 'lib.h'],
    command: [fastc, 'compile', '@INPUT@', '-o', '@OUTPUT0@', '--emit-header'],
)

# Static library
mylib_static = static_library('mylib', lib_c,
    include_directories: fastc_runtime,
    install: true,
)

# Shared library
mylib_shared = shared_library('mylib', lib_c,
    include_directories: fastc_runtime,
    install: true,
)

# Install headers
install_headers(lib_c[1])

# Dependency for other projects
mylib_dep = declare_dependency(
    link_with: mylib_static,
    include_directories: include_directories('.'),
)
```

## CI/CD Integration

### GitHub Actions

```yaml
name: Build

on: [push, pull_request]

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Install Rust
        uses: dtolnay/rust-action@stable

      - name: Build fastc
        run: |
          git clone https://github.com/Skelf-Research/fastc.git /tmp/fastc
          cd /tmp/fastc
          cargo build --release
          echo "FASTC_RUNTIME=/tmp/fastc/runtime" >> $GITHUB_ENV
          echo "/tmp/fastc/target/release" >> $GITHUB_PATH

      - name: Build project
        run: |
          fastc build --cc

      - name: Run tests
        run: ./build/main
```

### GitLab CI

```yaml
build:
  image: rust:latest
  script:
    - git clone https://github.com/Skelf-Research/fastc.git /tmp/fastc
    - cd /tmp/fastc && cargo build --release
    - export PATH="/tmp/fastc/target/release:$PATH"
    - export FASTC_RUNTIME=/tmp/fastc/runtime
    - cd $CI_PROJECT_DIR
    - fastc build --cc
  artifacts:
    paths:
      - build/
```

## Tips

1. **Set FASTC_RUNTIME** - Ensure the runtime path is configured
2. **Declare dependencies** - Make C files depend on .fc sources
3. **Use --emit-header** - Generate headers for library projects
4. **Cache fastc builds** - In CI, cache the compiled fastc binary
5. **Version lock fastc** - Use specific versions for reproducibility

## See Also

- [Project Management](../cli/project.md) - fastc.toml configuration
- [Build & Run](../cli/build-run.md) - CLI build commands
