#!/usr/bin/env bash
set -e

# Get the target from the first argument, default to "pico" if none provided
TARGET=${1:-pico}

if [[ "$TARGET" != "pico" && "$TARGET" != "linux" ]]; then
  echo "❌ Invalid target: $TARGET"
  echo "Usage: ./build.sh [pico|linux]"
  exit 1
fi

echo "========================================"
echo "🎯 Target: $TARGET"
echo "========================================"

echo
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

if [ "$TARGET" = "pico" ]; then
  echo
  echo "========================================"
  echo "🧩 Running compiler (Pulley)"
  echo "========================================"
  cargo run -p compiler -- unknown

  echo
  echo "========================================"
  echo "🚀 Running pico2 firmware (release)"
  echo "========================================"
  cd host
  cargo run --release
  cd ..
else
  echo
  echo "========================================"
  echo "🚀 Running Linux host (release)"
  echo "========================================"
  # Run the linux host directly from the root workspace
  cargo run -p linux-host --release -- --policy-file linux-host/policy.toml guest.component.wasm
fi

echo
echo "========================================"
echo "✅ All steps completed successfully"
echo "========================================"
