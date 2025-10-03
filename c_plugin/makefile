# Makefile for debug plugin dynamic library

# Compiler
CC = clang

# Target architecture (for macOS)
ARCH = arm64

# Output library name
TARGET = libplugin.dylib

# Source files
SOURCES = plugin.c

# Object files
OBJECTS = $(SOURCES:.c=.o)

# Compiler flags
CFLAGS = -Wall -Wextra -O2 -fPIC -arch $(ARCH)

# Linker flags for creating a dynamic library on macOS
LDFLAGS = -dynamiclib -arch $(ARCH)


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

# Install (optional - adjust path as needed)
install: $(TARGET)
	cp $(TARGET) /usr/local/lib/
	@echo "Installed $(TARGET) to /usr/local/lib/"

# Show information about the built library
info: $(TARGET)
	file $(TARGET)
	otool -L $(TARGET)


.PHONY: all clean rebuild install info test
