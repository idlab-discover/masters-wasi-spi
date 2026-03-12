use std::path::Path;
use std::{env, fs};
use wasmtime::{Config, Engine};

fn main() -> anyhow::Result<()> {
    println!("Compiling guest for Pulley...");
    let mode = env::args().nth(1).unwrap_or_else(|| "unknown".to_string());

    let input_path = match mode.as_str() {
        "p2" => Path::new("target/wasm32-wasip2/release/guest.wasm"),
        "unknown" => Path::new("guest.component.wasm"),
        _ => anyhow::bail!("Invalid mode '{}'. Use: p2 | unknown", mode),
    };

    // 1. Configure Engine to EXACTLY match the Pico 2 Host capabilities
    let mut config = Config::new();
    config.target("pulley32")?;

    // --- Features: Must match what is enabled/disabled in Host Cargo.toml ---
    config.wasm_component_model(true);
    config.async_support(false);

    // Disable GC features (Proposal + Support)
    config.wasm_gc(false);
    config.wasm_function_references(false);
    config.gc_support(false);

    // --- Runtime/Memory: Must match Host config initialization ---
    config.signals_based_traps(false); // No OS signals on Pico
    config.memory_init_cow(false); // Match CoW setting (Copy-on-Write)
    config.memory_guard_size(0); // No virtual memory guard pages
    config.memory_reservation(0); // No address space reservation
    config.max_wasm_stack(32 * 1024); // Match stack size limit

    let engine = Engine::new(&config)?;

    println!("Reading component from: {:?}", input_path);
    let wasm_bytes = fs::read(input_path)?;

    //println!("Componentizing module...");
    //let component_bytes = ComponentEncoder::default()
    //    .validate(true)
    //    .module(&wasm_bytes)?
    //    .encode()?;

    // 3. Precompile
    let serialized = engine.precompile_component(&wasm_bytes)?;

    // 4. Output
    let output_path = Path::new("host/src/guest.pulley");
    fs::write(output_path, &serialized)?;

    println!(
        "Success! Wrote {} bytes to {:?}",
        serialized.len(),
        output_path
    );
    Ok(())
}
