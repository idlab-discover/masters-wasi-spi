// example-host-component/src/lib.rs (or main.rs)

use anyhow::Result;
use wasmtime::{Engine, Store, component::{Component, Linker, ResourceTable}};
// Import generated bindings if needed, or just use your library
use wasi_spi_host::{SpiContext, add_to_linker};
use linux_embedded_hal::Spidev;

// 1. Define Host State
struct AppState {
    table: ResourceTable,
    // ADD THIS: We must hold the SPI context so Wasmtime can call read/write on it
    spi_ctx: SpiContext<Spidev>,
}

// Implement the helper trait if your library requires it (optional based on implementation)
// impl WasiView for AppState { ... }

// 2. Generate bindings
wasmtime::component::bindgen!({
    path: "../example-guest-component/wit",
    world: "app",
    with: { "my:hardware/spi/spi-device": wasmtime::component::ResourceAny }
});

fn main() -> Result<()> {
    let engine = Engine::default();
    let mut linker = Linker::new(&engine);

    // --- A. SETUP LIBRARY ---
    // FIX 1: Pass 'SpiContext<Spidev>', not 'Spidev'
    // FIX 2: The closure must return the Context, not the Table
    add_to_linker::<AppState, SpiContext<Spidev>>(&mut linker, |state| &mut state.spi_ctx)?;

    // --- B. SETUP HARDWARE ---
    let mut spi = Spidev::open("/dev/spidev0.0")?;

    // --- C. SETUP WASMTIME ---
    // Wrap the hardware
    let ctx = SpiContext { bus: spi };

    // Create state WITH the context
    // Note: We need to Clone ctx or use a RefCell if we want to put it in the table AND keep it in struct.
    // However, for the 'Dependency Injection' pattern, the Host usually owns the resource.
    // For simplicity here, we can rely on the Context being in the table for the *Guest* to use,
    // but the Linker needs access to the logic.

    // SIMPLIFICATION:
    // If 'add_to_linker' registers the 'HostSpiDevice' trait, it needs access to the struct implementing it.
    // We will put one instance in the struct (for the trait calls) and one in the table?
    // No, that won't work with hardware (cannot open twice).

    // CORRECT PATTERN:
    // The ResourceTable holds the data. The 'Host' trait methods receive 'Resource<T>'.
    // They look up the data in the table.
    // So 'add_to_linker' only needs access to the TABLE.

    // REVERTING TO YOUR TABLE CLOSURE:
    // If your library's 'add_to_linker' was implemented as:
    // fn add_to_linker(..., get_table: fn() -> Table)
    // Then your original code `|state| &mut state.table` WAS correct.

    // Let's assume you used the "Fix 3" from the previous turn (exposing bindings directly).
    // Then the call should be:
    wasi_spi_host::bindings::my::hardware::spi::add_to_linker(
        &mut linker,
        |state: &mut AppState| &mut state.table // This works if implemented correctly!
    )?;

    let mut state = AppState {
        table: ResourceTable::new(),
        // spi_ctx: ctx // Not needed if everything goes via table
    };

    // Push to table (Move semantics)
    let spi_resource = state.table.push(ctx)?;

    let mut store = Store::new(&engine, state);
    let component = Component::from_file(&engine, "../target/wasm32-wasi/debug/spi_guest.wasm")?;

    let (bindings, _) = App::instantiate(&mut store, &component, &linker)?;

    bindings.call_run(&mut store, spi_resource)?;

    Ok(())
}