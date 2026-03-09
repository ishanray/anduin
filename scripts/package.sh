#!/usr/bin/env bash
set -euo pipefail

APP_NAME="Anduin"
IDENTITY="Developer ID Application: Ishan Raychaudhuri (4RGA345X8U)"
ENTITLEMENTS="packaging/macos/entitlements.plist"
BUNDLE_PATH="target/release/bundle/osx/${APP_NAME}.app"

cd "$(dirname "$0")/.."

VERSION=$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)"/\1/')
DMG_NAME="${APP_NAME}-${VERSION}.dmg"
DIST_DIR="dist"

echo "==> Building release..."
cargo build --release

echo "==> Bundling..."
cargo bundle --release

echo "==> Signing app..."
codesign --force --options runtime --timestamp --deep \
  --sign "${IDENTITY}" \
  --entitlements "${ENTITLEMENTS}" \
  "${BUNDLE_PATH}"

echo "==> Verifying signature..."
codesign --verify --verbose=2 "${BUNDLE_PATH}"

echo "==> Creating DMG..."
mkdir -p "${DIST_DIR}"
DMG_PATH="${DIST_DIR}/${DMG_NAME}"
rm -f "${DMG_PATH}"

# Create a temporary directory for DMG contents
STAGING=$(mktemp -d)
trap 'rm -rf "${STAGING}"' EXIT

cp -R "${BUNDLE_PATH}" "${STAGING}/"
ln -s /Applications "${STAGING}/Applications"

hdiutil create -volname "${APP_NAME}" \
  -srcfolder "${STAGING}" \
  -ov -format UDZO \
  "${DMG_PATH}"

echo "==> Signing DMG..."
codesign --force --timestamp \
  --sign "${IDENTITY}" \
  "${DMG_PATH}"

echo "==> Verifying DMG signature..."
codesign --verify --verbose=2 "${DMG_PATH}"

echo ""
echo "Done: ${DMG_PATH}"
echo "Size: $(du -h "${DMG_PATH}" | cut -f1)"
