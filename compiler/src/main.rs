use std::env;
use std::fs;
use std::path::Path;
use wasmtime::{Config, Engine};

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: {} <input_wasm> <output_pulley>", args[0]);
        std::process::exit(1);
    }

    let input_path = Path::new(&args[1]);
    let output_path = Path::new(&args[2]);

    println!("Compiling {:?} for Pulley...", input_path);

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

    // 3. Read input, precompile, and write output
    let wasm_bytes = fs::read(input_path)?;
    let serialized = engine.precompile_component(&wasm_bytes)?;

    fs::write(output_path, &serialized)?;

    println!(
        "Success! Wrote {} bytes to {:?}",
        serialized.len(),
        output_path
    );

    Ok(())
}
