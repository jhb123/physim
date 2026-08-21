#!/usr/bin/env bash
set -e  # Exit immediately on error
set -u  # Treat unset variables as errors
set -o pipefail

# -------------------------
# Configuration
# -------------------------
# Where to install final binaries/plugins (make sure this is in your PATH)
INSTALL_DIR="${1:-$HOME}/physim"

SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )

# -------------------------
# Create install directory
# -------------------------
if [[ -d "$INSTALL_DIR" && "$(ls -A "$INSTALL_DIR")" ]]; then
    echo "The directory '$INSTALL_DIR' already exists and is not empty."
    read -p "Do you want to overwrite its contents? [y/N]: " -r
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo "Installation cancelled."
        exit 1
    fi

    echo "Clearing existing contents in $INSTALL_DIR..."
    find "$INSTALL_DIR" -mindepth 1 -maxdepth 1 -exec rm -r -- {} +
fi

mkdir -p "$INSTALL_DIR"
echo "Installing final binaries/plugins to: $INSTALL_DIR"
cp -r "$SCRIPT_DIR/physim"/* $INSTALL_DIR

# -------------------------
# Add INSTALL_DIR to PATH if not already
# -------------------------
if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
    SHELL_NAME=$(basename "$SHELL")
    case "$SHELL_NAME" in
        bash)
            SHELL_RC="$HOME/.bashrc"
            ;;
        zsh)
            SHELL_RC="$HOME/.zshrc"
            ;;
        *)
            SHELL_RC="$HOME/.profile"
            ;;
    esac

    echo "Adding $INSTALL_DIR to PATH in $SHELL_RC"
    echo "" >> "$SHELL_RC"
    echo "# Added by Physim installer" >> "$SHELL_RC"
    echo "export PATH=\"\$PATH:$INSTALL_DIR\"" >> "$SHELL_RC"

    echo "PATH updated! Please restart your terminal or run:"
    echo "   source $SHELL_RC"
else
    echo "$INSTALL_DIR is already in your PATH."
fi
