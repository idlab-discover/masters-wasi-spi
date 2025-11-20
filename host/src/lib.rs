use anyhow::Result;
use wasmtime::{
    Engine,
    component::{Linker, ResourceTable},
};
use wasmtime_wasi::{WasiCtx, WasiView};

pub mod spi;
pub use spi::SpiImplementation;

wasmtime::component::bindgen!({
    path: "../wit",
    world: "app",
});

pub struct HostState {
    pub table: ResourceTable,
    pub wasi: WasiCtx,
    pub spi_device: SpiImplementation,
}

impl HostState {
    pub fn new(spi_device: SpiImplementation, wasi: WasiCtx) -> Self {
        Self {
            table: ResourceTable::new(),
            wasi,
            spi_device,
        }
    }
}

impl WasiView for HostState {
    fn table(&mut self) -> &mut ResourceTable {
        &mut self.table
    }
    fn ctx(&mut self) -> &mut WasiCtx {
        &mut self.wasi
    }
}

pub fn setup_linker(engine: &Engine) -> Result<Linker<HostState>> {
    let mut linker = Linker::new(engine);

    wasmtime_wasi::add_to_linker_sync(&mut linker)?;
    my_org::hardware::spi::add_to_linker(&mut linker, |state: &mut HostState| {
        &mut state.spi_device
    })?;

    Ok(linker)
}
