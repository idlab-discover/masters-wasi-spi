use anyhow::Result;
use wasi_spi::SpiContext;
use wasmtime::component::{Component, HasSelf, Linker, Resource, ResourceTable};
use wasmtime::{Config, Engine, Store};
use wasmtime_wasi::p2::add_to_linker_sync;
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView};

wasmtime::component::bindgen!({
    path: "../example-guest-component/wit",
    world: "app",
    with: {
        "my:hardware/spi": wasi_spi::bindings::my::hardware::spi,
    }
});

mod mock_spi;
use mock_spi::MockSpiDevice;

use crate::my::hardware::spi::SpiDevice as GuestSpiDevice;

pub struct HostState {
    ctx: WasiCtx,
    table: ResourceTable,
    spi_context: SpiContext<MockSpiDevice>,
}

impl WasiView for HostState {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.ctx,
            table: &mut self.table,
        }
    }
}

pub fn main() -> Result<()> {
    let guest_path = "../target/wasm32-wasip2/release/example_guest_component.wasm";

    let mut config = Config::new();
    config.wasm_component_model(true);
    let engine = Engine::new(&config)?;
    let mut linker = Linker::new(&engine);

    add_to_linker_sync(&mut linker)?;

    wasi_spi::bindings::my::hardware::spi::add_to_linker::<
        HostState,
        HasSelf<SpiContext<MockSpiDevice>>,
    >(&mut linker, |state: &mut HostState| &mut state.spi_context)?;

    // let spi_bus = SpidevDevice::open("/dev/spidev0.0").context("Failed to open /dev/spidev0.0")?;

    let state = HostState {
        ctx: WasiCtxBuilder::new().inherit_stdio().build(),
        table: ResourceTable::new(),
        spi_context: SpiContext { bus: MockSpiDevice },
    };

    let mut store = Store::new(&engine, state);
    let component = Component::from_file(&engine, guest_path)?;
    let instance = App::instantiate(&mut store, &component, &linker)?;

    let device_resource_rep = 0;
    let device_handle = Resource::<GuestSpiDevice>::new_own(device_resource_rep);

    instance.call_run(&mut store, device_handle)?;

    Ok(())
}
