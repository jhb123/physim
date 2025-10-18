#!/usr/bin/env bash
set -e  # Exit immediately on error
set -u  # Treat unset variables as errors
set -o pipefail

# -------------------------
# Configuration
# -------------------------
# Where to install final binaries/plugins (make sure this is in your PATH)
INSTALL_DIR="${1:-$HOME/physim}"

# Rust project root (adjust if needed)
RUST_PROJECT_DIR="."

# C plugin directory (adjust if needed)
C_PLUGIN_DIR="c_plugin"

# -------------------------
# Create install directory
# -------------------------
mkdir -p "$INSTALL_DIR"
echo "Installing final binaries/plugins to: $INSTALL_DIR"

# -------------------------
# 1. Build Rust project
# -------------------------
echo "Building Rust project in release mode..."
cargo build --release --manifest-path "$RUST_PROJECT_DIR/Cargo.toml" --bin physcan --bin physim --lib

# Copy Rust binaries to install dir
echo "Copying Rust binaries..."
# Find all binaries in target/release
for bin in "$RUST_PROJECT_DIR/target/release/"*; do
    if [[ -f "$bin" && -x "$bin" ]]; then
        echo "Installing $(basename "$bin")"
        cp "$bin" "$INSTALL_DIR/"
    fi
done


echo "✅ Build and install complete. Make sure $INSTALL_DIR is in your PATH."
