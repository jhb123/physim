# Makefile for the python plugin's dynamic library
#
# This links a compiled CPython interpreter into the plugin, so the actual
# physics can live in gas.py instead of C. python3-config supplies the
# right include/link flags for whichever Python 3 is on PATH.

# Compiler
CC = clang

# Target architecture (for macOS)
ARCH = arm64

# Output library name
TARGET = libgas_plugin.dylib

# Source files
SOURCES = plugin.c

# Object files
OBJECTS = $(SOURCES:.c=.o)

# Python 3 embedding flags
PYTHON_CFLAGS = $(shell python3-config --includes)
PYTHON_LDFLAGS = $(shell python3-config --ldflags --embed 2>/dev/null || python3-config --ldflags)

# Compiler flags
CFLAGS = -Wall -Wextra -O2 -fPIC -arch $(ARCH) $(PYTHON_CFLAGS)

# Linker flags for creating a dynamic library on macOS
LDFLAGS = -dynamiclib -arch $(ARCH) -L../target/release -lphysim_core $(PYTHON_LDFLAGS)

# Libraries to link against (if needed)
LIBS =

# Default target
all: $(TARGET)

# Build the dynamic library
$(TARGET): $(OBJECTS)
	$(CC) $(LDFLAGS) $(LIBDIRS) -o $@ $(OBJECTS) $(LIBS)
	@echo "Built $(TARGET)"

# Compile source files to object files
%.o: %.c
	$(CC) $(CFLAGS) $(INCLUDES) -c $< -o $@

# Clean build artifacts
clean:
	rm -f $(OBJECTS) $(TARGET)
	@echo "Cleaned build artifacts"

# Rebuild everything
rebuild: clean all

# Install (also copies gas.py alongside the library, since the plugin
# locates it at runtime relative to its own path)
install: $(TARGET)
	cp $(TARGET) ../target/release
	cp gas.py ../target/release
	@echo "Installed $(TARGET) and gas.py to ../target/release"

# Show information about the built library
info: $(TARGET)
	file $(TARGET)
	otool -L $(TARGET)

.PHONY: all clean rebuild install info
