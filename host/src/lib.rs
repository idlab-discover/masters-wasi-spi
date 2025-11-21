use anyhow::Result;
use wasmtime::{
    Engine,
    component::{HasSelf, Linker, ResourceTable},
};
use wasmtime_wasi::{WasiCtx, WasiCtxView, WasiView};

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
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.wasi,
            table: &mut self.table,
        }
    }
}

pub fn setup_linker(engine: &Engine) -> Result<Linker<HostState>> {
    let mut linker = Linker::new(engine);

    wasmtime_wasi::p2::add_to_linker_sync(&mut linker)?;
    my_org::hardware::spi::add_to_linker::<HostState, HasSelf<SpiImplementation>>(
        &mut linker,
        |state: &mut HostState| &mut state.spi_device,
    )?;

    Ok(linker)
}
