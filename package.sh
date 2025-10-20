#!/usr/bin/env bash
set -euo pipefail

# ---- CONFIG ----
APP_NAME="physim"
VERSION="${GITHUB_REF_NAME:-dev}"
OUTDIR="dist/${APP_NAME}-${VERSION}-macos"

# ---- BUILD ----
echo "Building release binaries..."
cargo build --release

# ---- PACKAGE ----
echo "Packaging $APP_NAME version $VERSION..."
mkdir -p "${OUTDIR}/${APP_NAME}"

# Copy binaries
cp target/release/physim "${OUTDIR}/${APP_NAME}/"
cp target/release/physcan "${OUTDIR}/${APP_NAME}/"

# Copy dylib plugins
cp target/release/*.dylib "${OUTDIR}/${APP_NAME}/"

# Create tarball
tar czvf "dist/${APP_NAME}-${VERSION}-macos.tar.gz" -C dist "${APP_NAME}-${VERSION}-macos"

echo "✅ Created dist/${APP_NAME}-${VERSION}-macos.tar.gz"
