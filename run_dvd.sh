#!/bin/bash
set -e

# Define Paths
TARGET_DIR="target/wasm32-wasip2/release"
DRIVER_WASM="$TARGET_DIR/pmod_oled_driver.wasm"
DVD_WASM="$TARGET_DIR/ball_screensaver.wasm"
FINAL_WASM="$TARGET_DIR/dvd_final.wasm"

# Policy file location
POLICY_FILE="guests/oled-screen/policies.toml"

echo "========================================"
echo "💿  Building DVD Screensaver..."
echo "========================================"

# 1. Build Components
echo "  🔨 Building Driver..."
cargo build -p pmod-oled-driver --target wasm32-wasip2 --release

echo "  🔨 Building DVD Bounce..."
cargo build -p ball-screensaver --target wasm32-wasip2 --release

# 2. Compose
echo "  🔌 Composing to $FINAL_WASM..."
wac plug "$DVD_WASM" --plug "$DRIVER_WASM" -o "$FINAL_WASM"

# 3. Run Host
echo "  🚀 Running Host..."
echo "========================================"
cargo run -p host -- \
  "$FINAL_WASM" \
  --policy-file "$POLICY_FILE"
