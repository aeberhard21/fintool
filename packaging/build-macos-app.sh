#!/bin/bash
set -e

APP_NAME="Fintool"
BINARY_NAME="fintool"
VERSION=$(cargo metadata --no-deps --format-version=1 | jq -r '.packages[] | select(.name=="'"$BINARY_NAME"'") | .version')

APP_DIR="dist/${APP_NAME}.app"

# Build Rust binary
cargo build --release --features="ratatui_support"

# Clean & create bundle dirs
rm -rf "$APP_DIR"
mkdir -p "$APP_DIR/Contents/MacOS"
mkdir -p "$APP_DIR/Contents/Resources"

# Copy binary
cp "target/release/${BINARY_NAME}" "$APP_DIR/Contents/MacOS/"

# Copy launcher
cp packaging/launcher.sh "$APP_DIR/Contents/MacOS/${BINARY_NAME}_launcher"
chmod +x "$APP_DIR/Contents/MacOS/${BINARY_NAME}_launcher"

# Fill in Info.plist template
sed \
  -e "s/__APP_NAME__/${APP_NAME}/g" \
  -e "s/__BINARY_NAME__/${BINARY_NAME}/g" \
  -e "s/__VERSION__/${VERSION}/g" \
  packaging/Info.plist > "$APP_DIR/Contents/Info.plist"

# Copy icon if present
if [ -f packaging/icon.icns ]; then
    cp packaging/icon.icns "$APP_DIR/Contents/Resources/icon.icns"
fi

echo "✅ Built $APP_DIR"