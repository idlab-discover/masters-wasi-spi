#!/usr/bin/env bash
set -e

# Get the target from the first argument, default to "pico" if none provided
TARGET=${1:-pico}

if [[ "$TARGET" != "pico" && "$TARGET" != "linux" && "$TARGET" != "bench-linux-wasm" && "$TARGET" != "bench-linux-native" ]]; then
  echo "❌ Invalid target: $TARGET"
  echo "Usage: ./build.sh [pico|linux|bench-linux-wasm|bench-linux-native]"
  exit 1
fi

echo "========================================"
echo "🎯 Target: $TARGET"
echo "========================================"

# --- NATIVE LINUX BENCHMARK ---
if [[ "$TARGET" == "bench-linux-native" ]]; then
  echo
  echo "========================================"
  echo "🚀 Running Native Linux Benchmark"
  echo "========================================"
  cargo run -p native-linux-host --release

  echo
  echo "========================================"
  echo "✅ Native benchmark completed successfully"
  echo "========================================"
  exit 0
fi

# --- WASM LINUX BENCHMARK ---
if [[ "$TARGET" == "bench-linux-wasm" ]]; then
  echo
  echo "========================================"
  echo "🛠  Building benchmark guest crate (wasm32 target)"
  echo "========================================"
  # Assumes the package name in benchmark/guest/Cargo.toml is 'benchmark-guest'
  cargo build -p benchmark-guest --target wasm32-unknown-unknown --release

  echo
  echo "========================================"
  echo "📦 Creating Benchmark WASM component"
  echo "========================================"
  wasm-tools component new \
    target/wasm32-unknown-unknown/release/benchmark_guest.wasm \
    -o target/wasm32-unknown-unknown/release/benchmark_guest.component.wasm

  echo
  echo "========================================"
  echo "🚀 Running WASM Linux Benchmark"
  echo "========================================"
  cargo run -p wasm-linux-host --release

  echo
  echo "========================================"
  echo "✅ WASM benchmark completed successfully"
  echo "========================================"
  exit 0
fi

# --- STANDARD PICO / LINUX RUN ---
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
elif [ "$TARGET" = "linux" ]; then
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
