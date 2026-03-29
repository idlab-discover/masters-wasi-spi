#!/usr/bin/env bash
set -e

# Get the target from the first argument, default to "pico" if none provided
TARGET=${1:-pico}

if [[ "$TARGET" != "pico" && "$TARGET" != "linux" && "$TARGET" != "bench-linux" && "$TARGET" != "bench-pico" ]]; then
  echo "❌ Invalid target: $TARGET"
  echo "Usage: ./build.sh [pico|linux|bench-linux|bench-pico]"
  exit 1
fi

echo "========================================"
echo "🎯 Target: $TARGET"
echo "========================================"

# --- UNIFIED LINUX BENCHMARK ---
if [[ "$TARGET" == "bench-linux" ]]; then
  echo
  echo "========================================"
  echo "🛠  Building benchmark guest crate (wasm32 target)"
  echo "========================================"
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
  echo "🚀 Running Unified Linux Benchmark (Native + WASM)"
  echo "========================================"
  cargo run -p benchmark-linux-host --release

  echo
  echo "========================================"
  echo "✅ Benchmark matrix completed successfully"
  echo "========================================"
  exit 0
fi

# --- UNIFIED PICO BENCHMARK ---
if [[ "$TARGET" == "bench-pico" ]]; then
  echo
  echo "========================================"
  echo "🛠  Building benchmark guest crate (wasm32 target)"
  echo "========================================"
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
  echo "🧩 Running compiler (Pulley) for Benchmark"
  echo "========================================"
  # Pass input and output dynamically
  cargo run -p compiler -- target/wasm32-unknown-unknown/release/benchmark_guest.component.wasm benchmark/pico-host/src/benchmark_guest.pulley

  echo
  echo "========================================"
  echo "🚀 Running Unified Pico Benchmark (Native + WASM)"
  echo "========================================"
  # Enter the specific host directory so `.cargo/config.toml` is applied!
  cd benchmark/pico-host
  cargo run --release
  cd ../..

  echo
  echo "========================================"
  echo "✅ Benchmark matrix completed successfully"
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
  # Pass standard input and output dynamically
  cargo run -p compiler -- guest.component.wasm host/src/guest.pulley

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
  cargo run -p linux-host --release -- --policy-file linux-host/policy.toml guest.component.wasm
fi

echo
echo "========================================"
echo "✅ All steps completed successfully"
echo "========================================"
