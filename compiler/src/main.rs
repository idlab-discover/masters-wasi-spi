use std::fs;
use std::path::Path;
use wasmtime::{Config, Engine};

fn main() -> anyhow::Result<()> {
    println!("Compiling guest for Pulley...");

    // Configure compiler options for compilation to Pulley on an embedded system
    let mut config = Config::new();
    config.target("pulley32")?;
    config.wasm_component_model(true);
    config.async_support(false);
    config.wasm_gc(false);
    config.wasm_function_references(false);
    config.gc_support(false);
    config.signals_based_traps(false);
    config.memory_init_cow(false);
    config.memory_guard_size(0);
    config.memory_reservation(0);
    config.max_wasm_stack(32 * 1024);
    let engine = Engine::new(&config)?;

    let input_path = Path::new("guest.component.wasm");
    let wasm_bytes = fs::read(input_path)?;
    let serialized = engine.precompile_component(&wasm_bytes)?;

    let output_path = Path::new("host/src/guest.pulley");
    fs::write(output_path, &serialized)?;

    println!(
        "Success! Wrote {} bytes to {:?}",
        serialized.len(),
        output_path
    );
    Ok(())
}
