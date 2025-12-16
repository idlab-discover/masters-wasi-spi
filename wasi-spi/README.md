# wasi-spi

This library provides a reusable **Wasmtime host-side implementation** of the SPI (Serial Peripheral Interface) WIT interface defined in the `wit` folder.

It allows WebAssembly components to communicate with physical SPI devices on Linux (via `linux-embedded-hal`) while running inside a Wasmtime host.

## Design Philosophy

**`wasi-spi` is not a standalone host component.** Instead, it is designed as a library that exposes a **Trait** (`WasiSpiView`) that any Wasmtime host can implement.

### Why this approach?
If we implemented the WIT interface directly on a specific `Host` struct, the implementation would be tightly coupled to that specific application. Other developers who want to build their own custom hosts (maybe combining SPI with other features like I2C or HTTP) would not be able to reuse the SPI logic.

By exposing the implementation via a trait, we decouple the **SPI capability** from the **Host application**. 

## Usage

To add SPI capabilities to your Wasmtime host, follow these steps:


### 1. Add the library as a dependency

Add the following line to your `Cargo.toml` dependencies:

```toml
[dependencies]
wasi-spi = { git = "https://github.com/idlab-discover/masters-wasi-spi", subdir = "wasi-spi" }
```

### 2. Implement `WasiSpiView`
Your host state struct must implement the `WasiSpiView` trait. This trait requires you to provide mutable access to a `WasiSpiCtx`.

```rust
use wasi_spi::{WasiSpiCtx, WasiSpiView};

struct MyHost {
    spi_ctx: WasiSpiCtx,
    // ... other WASI contexts
}

impl WasiSpiView for MyHost {
    fn spi_ctx(&mut self) -> &mut WasiSpiCtx {
        &mut self.spi_ctx
    }
}
```

### 3. Add to Linker
Register the SPI implementation with your Wasmtime `Linker`. This binds the WIT interface functions (guest side) to the Rust implementation (host side).

```rust
// In your main host setup
wasi_spi::add_to_linker(&mut linker)?;
```

## Configuration

The `WasiSpiCtx` is the core context object that manages the open SPI devices. It is instantiated using a configuration list that maps physical hardware to secure virtual names.

This mapping is important for security and portability:
* **Physical Path:** The actual device file on the Linux host (e.g., `/dev/spidev0.0`).
* **Virtual Name:** The simple string the Wasm guest uses to request the device (e.g., `"screen"` or `"sensor"`).

### Example Setup

```rust
use wasi_spi::{SpiConfig, WasiSpiCtx};

let spi_configs = vec![
    SpiConfig {
        physical_path: "/dev/spidev0.0".to_string(),
        virtual_name: "display".to_string(),
    }
];

// Create the context
let spi_ctx = WasiSpiCtx::from_configs(spi_configs)?;
```

In this example, the Wasm guest would open the device named `"display"`, and the host handles the translation to `/dev/spidev0.0` transparently.
