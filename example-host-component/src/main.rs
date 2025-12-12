use anyhow::Context;
use clap::Parser;
use linux_embedded_hal::SpidevDevice;
use wasmtime::{
    Config, Engine, Store,
    component::{Component, Linker},
};
use wasmtime_wasi::{ResourceTable, WasiCtx, WasiCtxBuilder, WasiView, p2::add_to_linker_sync};

use wasi_spi::{SpiDeviceState, WasiSpiCtx, WasiSpiView};

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

    let mut state = HostState {
        ctx: WasiCtxBuilder::new().inherit_stdio().build(),
        table: ResourceTable::new(),
        spi_ctx: WasiSpiCtx::new(),
    };

    for device_config in args.devices {
        let physical_path = device_config.physical_path;
        let virtual_name = device_config.virtual_name;

        let physical = SpidevDevice::open(&physical_path)
            .with_context(|| format!("Unable to open device at {}", physical_path))?;

        let device_state = SpiDeviceState { device: physical };

        let handle = state.table.push(device_state)?;

        state.spi_ctx.devices.insert(virtual_name, handle);
    }

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
