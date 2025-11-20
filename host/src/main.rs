use anyhow::Result;
use clap::Parser;
use wasmtime::{component::Component, Config, Engine, Store};
use wasmtime_wasi::WasiCtxBuilder;
// Ensure these are public in your lib.rs
use host::{HostState, setup_linker, SpiImplementation};
// FIX: Import Spidev from the spidev submodule
use linux_embedded_hal::spidev::Spidev;

#[derive(Parser)]
struct Args {
    /// Path to the compiled guest wasm file
    #[arg(short, long)]
    wasm: String,

    /// Path to the SPI device (e.g., /dev/spidev0.0)
    #[arg(short, long, default_value = "/dev/spidev0.0")]
    device: String,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let mut config = Config::new();
    config.wasm_component_model(true);
    let engine = Engine::new(&config)?;

    let spi_dev = Spidev::open(&args.device)?;

    let mut store = Store::new(
        &engine,
        HostState::new(
            SpiImplementation(spi_dev),
            WasiCtxBuilder::new().inherit_stdout().build()
        ),
    );

    let linker = setup_linker(&engine)?;

    let component = Component::from_file(&engine, &args.wasm)?;
    let (app, _) = host::App::instantiate(&mut store, &component, &linker)?;

    println!("Host: Running guest...");
    app.call_run(&mut store)?;
    println!("Host: Guest execution finished.");

    Ok(())
}