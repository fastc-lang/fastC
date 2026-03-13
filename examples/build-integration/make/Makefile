# Makefile for FastC projects
#
# This Makefile demonstrates how to integrate FastC into a Make-based build.
# It compiles .fc files to .c files using fastc, then builds the final executable.

# Configuration
FASTC ?= fastc
CC ?= cc
CFLAGS ?= -std=c11 -Wall -Wextra

# Find all .fc source files
FC_SOURCES := $(wildcard *.fc) $(wildcard ../*.fc)
C_SOURCES := $(FC_SOURCES:.fc=.c)
OBJECTS := $(C_SOURCES:.c=.o)
TARGET := hello

.PHONY: all clean check fmt

all: $(TARGET)

# Link the final executable
$(TARGET): $(OBJECTS)
	$(CC) $(CFLAGS) -o $@ $^

# Compile C to object files
%.o: %.c
	$(CC) $(CFLAGS) -c -o $@ $<

# Transpile FastC to C
%.c: %.fc
	$(FASTC) compile $< -o $@

# Type-check all FastC sources without compiling
check:
	@for f in $(FC_SOURCES); do \
		echo "Checking $$f..."; \
		$(FASTC) check $$f || exit 1; \
	done
	@echo "All files pass type checking."

# Format all FastC sources
fmt:
	@for f in $(FC_SOURCES); do \
		echo "Formatting $$f..."; \
		$(FASTC) fmt $$f; \
	done

# Format check (verify files are formatted)
fmt-check:
	@for f in $(FC_SOURCES); do \
		$(FASTC) fmt $$f --check || exit 1; \
	done

clean:
	rm -f $(C_SOURCES) $(OBJECTS) $(TARGET)

# Prevent make from removing intermediate .c files
.PRECIOUS: %.c
