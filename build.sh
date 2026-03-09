#!/usr/bin/env bash
set -e

echo "========================================"
echo "🛠  Building guest crate (wasm32 target)"
echo "========================================"
cargo build -p guest --target wasm32-unknown-unknown --release

echo
echo "========================================"
echo "📦 Creating WASM component"
echo "========================================"
wasm-tools component new \
  target/wasm32-unknown-unknown/release/guest.wasm \
  -o guest.component.wasm

echo
echo "========================================"
echo "🧩 Running compiler"
echo "========================================"
cargo run -p compiler -- unknown

echo
echo "========================================"
echo "📁 Entering pico2-quick project"
echo "========================================"
cd pico2-quick

echo
echo "========================================"
echo "🚀 Running pico2 firmware (release)"
echo "========================================"
cargo run --release

echo
echo "========================================"
echo "✅ All steps completed successfully"
echo "========================================"
