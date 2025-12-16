#!/bin/bash
set -e

# Define Paths
TARGET_DIR="target/wasm32-wasip2/release"
DRIVER_WASM="$TARGET_DIR/pmod_oled_driver.wasm"
PACMAN_WASM="$TARGET_DIR/pacman.wasm"
FINAL_WASM="$TARGET_DIR/pacman_final.wasm"

# Policy file location
POLICY_FILE="guests/oled-screen/pmod-oled-driver/policies.toml"

echo "========================================"
echo "🕹️  Building Pacman Application..."
echo "========================================"

# 1. Build Components
echo "  🔨 Building Driver..."
cargo build -p pmod-oled-driver --target wasm32-wasip2 --release

echo "  🔨 Building Pacman..."
cargo build -p pacman --target wasm32-wasip2 --release

# 2. Compose
echo "  🔌 Composing to $FINAL_WASM..."
wac plug "$PACMAN_WASM" --plug "$DRIVER_WASM" -o "$FINAL_WASM"

# 3. Run Host
echo "  🚀 Running Host..."
echo "========================================"
cargo run -p host -- \
  "$FINAL_WASM" \
  --policy-file "$POLICY_FILE"
