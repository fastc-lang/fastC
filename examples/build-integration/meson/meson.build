# meson.build for FastC projects
#
# This meson.build demonstrates how to integrate FastC into a Meson-based build.
# It uses a custom generator to compile .fc files to .c files.

project('fastc_example', 'c',
    version: '0.1.0',
    default_options: ['c_std=c11', 'warning_level=2']
)

# Find the FastC compiler
fastc = find_program('fastc', required: true)

# Create a generator for FastC files
# This will automatically transpile .fc files to .c files
fastc_gen = generator(fastc,
    output: '@BASENAME@.c',
    arguments: ['compile', '@INPUT@', '-o', '@OUTPUT@']
)

# Process FastC source files
fc_sources = files('../hello.fc')
c_sources = fastc_gen.process(fc_sources)

# Build the executable
executable('hello', c_sources)

# Custom target for type-checking (optional)
run_target('check',
    command: [fastc, 'check', fc_sources]
)

# Custom target for formatting (optional)
run_target('fmt',
    command: [fastc, 'fmt', fc_sources]
)
