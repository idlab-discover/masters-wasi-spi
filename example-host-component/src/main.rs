use clap::Parser;
use wasmtime::{
    Config, Engine, Store,
    component::{Component, Linker},
};
use wasmtime_wasi::{ResourceTable, WasiCtx, WasiCtxBuilder, WasiView, p2::add_to_linker_sync};

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
}

impl WasiView for HostState {
    fn ctx(&mut self) -> wasmtime_wasi::WasiCtxView<'_> {
        wasmtime_wasi::WasiCtxView {
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

fn main() -> anyhow::Result<()> {
    let args = argument_parser::HostArguments::parse();

    let spi_configs: Vec<SpiConfig> = args
        .devices
        .into_iter()
        .map(|device| SpiConfig {
            virtual_name: device.virtual_name,
            physical_path: device.physical_path,
        })
        .collect();

    let spi_ctx = WasiSpiCtx::from_configs(spi_configs)?;

    let state = HostState {
        ctx: WasiCtxBuilder::new().inherit_stdio().build(),
        table: ResourceTable::new(),
        spi_ctx,
    };

    let mut config = Config::new();
    config.wasm_component_model(true);
    let engine = Engine::new(&config)?;
    let mut linker = Linker::new(&engine);

    add_to_linker_sync(&mut linker)?;

    wasi_spi::add_to_linker(&mut linker)?;

    let mut store = Store::new(&engine, state);
    let component = Component::from_file(&engine, args.component_path)?;

    let app = App::instantiate(&mut store, &component, &linker)?;

    println!("Host: Calling guest run()...");
    app.call_run(&mut store)?;
    println!("Host: Guest finished.");

    Ok(())
}
