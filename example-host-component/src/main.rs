use clap::Parser;
use wasmtime::{
    Config, Engine, Store,
    component::{Component, Linker},
};
use wasmtime_wasi::{
    ResourceTable, WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView, p2::add_to_linker_sync,
};

use wasi_spi::{SpiConfig, WasiSpiCtx, WasiSpiView};

// 1. Import wasi-gpio types
use wasi_gpio::{WasiGpioCtx, WasiGpioView};

mod argument_parser;

wasmtime::component::bindgen!({
    path: "../example-guest-component/wit",
    world: "app",
});

struct HostState {
    ctx: WasiCtx,
    table: ResourceTable,
    spi_ctx: WasiSpiCtx,
    // 2. Add GPIO Context to HostState
    gpio_ctx: WasiGpioCtx,
}

impl WasiView for HostState {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.ctx,
            table: &mut self.table,
        }
    }
}

impl WasiSpiView for HostState {
    fn spi_ctx(&mut self) -> &mut WasiSpiCtx {
        &mut self.spi_ctx
    }
}

// 3. Implement WasiGpioView for HostState
impl WasiGpioView for HostState {
    fn gpio_ctx(&mut self) -> &mut WasiGpioCtx {
        &mut self.gpio_ctx
    }

    fn table(&mut self) -> &mut ResourceTable {
        &mut self.table
    }
}

fn main() -> anyhow::Result<()> {
    let args = argument_parser::HostArguments::parse();

    // ... (Existing SPI setup code) ...
    let spi_configs: Vec<SpiConfig> = args
        .devices
        .into_iter()
        .map(|device| SpiConfig {
            virtual_name: device.virtual_name,
            physical_path: device.physical_path,
        })
        .collect();

    let spi_ctx = WasiSpiCtx::from_configs(spi_configs)?;

    // 4. Initialize GPIO Context
    // You might need to load this from a config file or arguments,
    // but here is a default initialization.
    // Ensure you have a 'policies.toml' if the library requires it,
    // or use a default configuration if available.
    let gpio_config = wasi_gpio::policies::Config::parse(); // Or construct manually
    let gpio_policies = gpio_config.get_policies();
    let gpio_ctx = WasiGpioCtx::new(gpio_policies);

    let state = HostState {
        ctx: WasiCtxBuilder::new().inherit_stdio().build(),
        table: ResourceTable::new(),
        spi_ctx,
        gpio_ctx, // Add to state
    };

    let mut config = Config::new();
    config.wasm_component_model(true);
    let engine = Engine::new(&config)?;
    let mut linker = Linker::new(&engine);

    // Add WASI defaults
    add_to_linker_sync(&mut linker)?;

    // Add SPI bindings
    wasi_spi::add_to_linker(&mut linker)?;

    // 5. Add GPIO bindings to linker
    wasi_gpio::add_to_linker(&mut linker)?;

    let mut store = Store::new(&engine, state);
    let component = Component::from_file(&engine, args.component_path)?;

    let app = App::instantiate(&mut store, &component, &linker)?;

    println!("Host: Calling guest run()...");
    app.call_run(&mut store)?;
    println!("Host: Guest finished.");

    Ok(())
}
