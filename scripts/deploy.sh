#!/usr/bin/env bash
set -euo pipefail

APP_NAME="Anduin"
BUNDLE_PATH="target/release/bundle/osx/${APP_NAME}.app"
INSTALL_PATH="/Applications/${APP_NAME}.app"

cd "$(dirname "$0")/.."

echo "Building release..."
cargo build --release

echo "Bundling..."
cargo bundle --release

# Quit running instance if any
osascript -e "tell application \"${APP_NAME}\" to quit" 2>/dev/null || true
sleep 1

echo "Installing to ${INSTALL_PATH}..."
rm -rf "${INSTALL_PATH}"
cp -R "${BUNDLE_PATH}" "${INSTALL_PATH}"

echo "Launching..."
open "${INSTALL_PATH}"

echo "Done."
