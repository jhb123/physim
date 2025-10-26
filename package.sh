#!/usr/bin/env bash
set -euo pipefail

# ---- CONFIG ----
APP_NAME="physim"
VERSION="${GITHUB_REF_NAME:-dev}"
PLATFORM="${1:-macos}"

# Map platform name to Rust target triple
case "${PLATFORM}" in
  macos)
    TARGET="x86_64-apple-darwin"
    LIB_EXT="dylib"
    BIN_EXT=""
    ;;
  linux)
    TARGET="x86_64-unknown-linux-gnu"
    LIB_EXT="so"
    BIN_EXT=""
    ;;
  windows)
    TARGET="x86_64-pc-windows-gnu"
    LIB_EXT="dll"
    BIN_EXT=".exe"
    ;;
  *)
    echo "❌ Unsupported platform: ${PLATFORM}" >&2
    exit 1
    ;;
esac

OUTDIR="dist/${APP_NAME}-${VERSION}-${PLATFORM}"
TARGET_DIR="target/${TARGET}/release"

# ---- BUILD ----
echo "🚧 Building ${APP_NAME} for ${PLATFORM} (${TARGET})..."
cargo build --release --target "${TARGET}"

# ---- PACKAGE ----
echo "📦 Packaging ${APP_NAME} version ${VERSION} for ${PLATFORM}..."
mkdir -p "${OUTDIR}/${APP_NAME}"

# Copy binaries
cp "${TARGET_DIR}/physim${BIN_EXT}" "${OUTDIR}/${APP_NAME}/"
cp "${TARGET_DIR}/physcan${BIN_EXT}" "${OUTDIR}/${APP_NAME}/"

# Copy dynamic libraries (if any)
if compgen -G "${TARGET_DIR}/*.${LIB_EXT}" > /dev/null; then
  cp "${TARGET_DIR}"/*.${LIB_EXT} "${OUTDIR}/${APP_NAME}/"
fi

# ---- ARCHIVE ----
TAR_NAME="dist/${APP_NAME}-${VERSION}-${PLATFORM}.tar.gz"
tar czvf "${TAR_NAME}" -C dist "${APP_NAME}-${VERSION}-${PLATFORM}"

echo "✅ Created ${TAR_NAME}"
