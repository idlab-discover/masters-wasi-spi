use clap::Parser;
use wasmtime::{
    Config, Engine, Store,
    component::{Component, Linker},
};
use wasmtime_wasi::{
    ResourceTable, WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView, p2::add_to_linker_sync,
};

use wasi_gpio::{WasiGpioCtx, WasiGpioView};
use wasi_spi::{SpiConfig, WasiSpiCtx, WasiSpiView};

mod argument_parser;

wasmtime::component::bindgen!({
    path: "../example-guest-component/wit",
    world: "app",
});

struct HostState {
    ctx: WasiCtx,
    table: ResourceTable,
    spi_ctx: WasiSpiCtx,
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

impl WasiGpioView for HostState {
    fn gpio_ctx(&mut self) -> &mut WasiGpioCtx {
        &mut self.gpio_ctx
    }

    fn table(&mut self) -> &mut ResourceTable {
        &mut self.table
    }
}

fn main() -> anyhow::Result<()> {
    // 1. Parse CLI arguments (now includes policy_file)
    let args = argument_parser::HostArguments::parse();

    // 2. Configure SPI Context
    let spi_configs: Vec<SpiConfig> = args
        .devices
        .into_iter()
        .map(|device| SpiConfig {
            virtual_name: device.virtual_name,
            physical_path: device.physical_path,
        })
        .collect();

    let spi_ctx = WasiSpiCtx::from_configs(spi_configs)?;

    // 3. Configure GPIO Context
    // We construct the Config struct manually to utilize its internal TOML parsing logic
    // without needing to add 'toml' as a direct dependency in this crate.
    let gpio_config = wasi_gpio::policies::Config {
        policy_file: args.policy_file,
        component: args.component_path.clone(), // Not strictly used for ctx creation, but required by struct
    };

    // Load policies from the file specified in CLI args
    let policies = gpio_config.get_policies();
    let gpio_ctx = WasiGpioCtx::new(policies);

    // 4. Build Host State
    let state = HostState {
        ctx: WasiCtxBuilder::new().inherit_stdio().build(),
        table: ResourceTable::new(),
        spi_ctx,
        gpio_ctx,
    };

    // 5. Setup Wasmtime Engine
    let mut config = Config::new();
    config.wasm_component_model(true);
    let engine = Engine::new(&config)?;
    let mut linker = Linker::new(&engine);

    // 6. Add Bindings to Linker
    // WASI Standard
    add_to_linker_sync(&mut linker)?;

    // SPI
    wasi_spi::add_to_linker(&mut linker)?;

    // GPIO
    wasi_gpio::add_to_linker(&mut linker)?;

    // 7. Instantiate and Run
    let mut store = Store::new(&engine, state);
    let component = Component::from_file(&engine, &args.component_path)?;

    let app = App::instantiate(&mut store, &component, &linker)?;

    println!("Host: Calling guest run()...");
    app.call_run(&mut store)?;
    println!("Host: Guest finished.");

    Ok(())
}
