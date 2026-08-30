# Makefile for physim C/C++ plugins

# Compilers
CC  = clang
CXX = clang++

# Target architecture (for macOS)
ARCH = arm64

# ── C plugin (libplugin.dylib) ────────────────────────────────────────────────

TARGET  = libplugin.dylib
SOURCES = plugin.c
OBJECTS = $(SOURCES:.c=.o)

CFLAGS  = -Wall -Wextra -O2 -fPIC -arch $(ARCH)
LDFLAGS = -dynamiclib -arch $(ARCH) -L../target/release -lphysim_core

# ── C++ geodesic plugin (libgeodesic.dylib) ───────────────────────────────────

GEODESIC_TARGET = libgeodesic.dylib
GEODESIC_SRC    = schwarzschild_geodesic.cpp
GEODESIC_OBJ    = schwarzschild_geodesic.o
VARITENSOR_H    = varitensor.h
VARITENSOR_URL  = https://raw.githubusercontent.com/BenDStokes/Vari-Tensor/main/varitensor.h

CXXFLAGS = -std=c++2b -Wall -Wextra -O2 -fPIC -arch $(ARCH)

# ── Default target ────────────────────────────────────────────────────────────

all: $(TARGET)

# ── C plugin rules ────────────────────────────────────────────────────────────

$(TARGET): $(OBJECTS)
	$(CC) $(LDFLAGS) -o $@ $(OBJECTS)
	@echo "Built $(TARGET)"

%.o: %.c
	$(CC) $(CFLAGS) $(INCLUDES) -c $< -o $@

# ── Vari-Tensor header ────────────────────────────────────────────────────────

$(VARITENSOR_H):
	curl -fsSL $(VARITENSOR_URL) -o $(VARITENSOR_H)
	@# Patch std::ranges::iota → std::iota for Apple Clang compatibility
	sed -i '' 's/std::ranges::iota(\(.*\), 0);/std::iota(\1.begin(), \1.end(), 0);/' $(VARITENSOR_H)
	@echo "Downloaded and patched $(VARITENSOR_H)"

# ── C++ geodesic plugin rules ─────────────────────────────────────────────────

$(GEODESIC_OBJ): $(GEODESIC_SRC) $(VARITENSOR_H)
	$(CXX) $(CXXFLAGS) -c $< -o $@

$(GEODESIC_TARGET): $(GEODESIC_OBJ)
	$(CXX) -dynamiclib -arch $(ARCH) -L../target/release -lphysim_core \
	    -o $@ $(GEODESIC_OBJ)
	@echo "Built $(GEODESIC_TARGET)"

geodesic: $(GEODESIC_TARGET)

varitensor.h: $(VARITENSOR_H)

# ── Utility targets ───────────────────────────────────────────────────────────

clean:
	rm -f $(OBJECTS) $(TARGET) $(GEODESIC_OBJ) $(GEODESIC_TARGET)
	@echo "Cleaned build artifacts"

rebuild: clean all

install: $(TARGET)
	cp $(TARGET) ../target/release
	@echo "Installed $(TARGET) to ../target/release"

install_geodesic: $(GEODESIC_TARGET)
	cp $(GEODESIC_TARGET) ../target/release
	@echo "Installed $(GEODESIC_TARGET) to ../target/release"

info: $(TARGET)
	file $(TARGET)
	otool -L $(TARGET)

info_geodesic: $(GEODESIC_TARGET)
	file $(GEODESIC_TARGET)
	otool -L $(GEODESIC_TARGET)

.PHONY: all clean rebuild install install_geodesic geodesic varitensor.h info info_geodesic
