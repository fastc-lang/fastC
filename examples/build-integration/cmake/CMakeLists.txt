# CMakeLists.txt for FastC projects
#
# This CMakeLists.txt demonstrates how to integrate FastC into a CMake-based build.
# It provides a custom function add_fastc_executable() that handles .fc files.

cmake_minimum_required(VERSION 3.16)
project(fastc_example C)

# Find the FastC compiler
find_program(FASTC_COMPILER fastc
    HINTS
        $ENV{HOME}/.cargo/bin
        /usr/local/bin
        /usr/bin
)

if(NOT FASTC_COMPILER)
    message(FATAL_ERROR "fastc compiler not found. Please install it or set FASTC_COMPILER.")
endif()

message(STATUS "Found FastC compiler: ${FASTC_COMPILER}")

# Function to add a FastC executable
# Usage: add_fastc_executable(target source1.fc source2.fc ...)
function(add_fastc_executable target)
    set(c_sources "")

    foreach(fc_source ${ARGN})
        # Get the absolute path and base name
        get_filename_component(fc_abs "${fc_source}" ABSOLUTE)
        get_filename_component(fc_name "${fc_source}" NAME_WE)

        # Output .c file in the binary directory
        set(c_output "${CMAKE_CURRENT_BINARY_DIR}/${fc_name}.c")

        # Add custom command to compile .fc to .c
        add_custom_command(
            OUTPUT "${c_output}"
            COMMAND "${FASTC_COMPILER}" compile "${fc_abs}" -o "${c_output}"
            DEPENDS "${fc_abs}"
            COMMENT "Transpiling ${fc_source} to C"
            VERBATIM
        )

        list(APPEND c_sources "${c_output}")
    endforeach()

    # Create the executable from generated C sources
    add_executable(${target} ${c_sources})
    target_compile_options(${target} PRIVATE -std=c11 -Wall -Wextra)
endfunction()

# Function to type-check FastC sources
# Usage: add_fastc_check(target source1.fc source2.fc ...)
function(add_fastc_check target)
    set(check_commands "")

    foreach(fc_source ${ARGN})
        get_filename_component(fc_abs "${fc_source}" ABSOLUTE)
        list(APPEND check_commands
            COMMAND "${FASTC_COMPILER}" check "${fc_abs}"
        )
    endforeach()

    add_custom_target(${target}
        ${check_commands}
        COMMENT "Type-checking FastC sources"
        VERBATIM
    )
endfunction()

# Example usage:
add_fastc_executable(hello ../hello.fc)
add_fastc_check(check ../hello.fc)
