use anyhow::Result;
use embedded_hal::spi::SpiDevice;
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

pub struct HostState<T> {
    pub table: ResourceTable,
    pub wasi: WasiCtx,
    pub spi_device: SpiImplementation<T>,
}

impl<T> HostState<T> {
    pub fn new(spi_device: SpiImplementation<T>, wasi: WasiCtx) -> Self {
        Self {
            table: ResourceTable::new(),
            wasi,
            spi_device,
        }
    }
}

impl<T: Send + Sync> WasiView for HostState<T> {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.wasi,
            table: &mut self.table,
        }
    }
}

pub fn setup_linker<T: SpiDevice + Send + Sync + 'static>(
    engine: &Engine,
) -> Result<Linker<HostState<T>>> {
    let mut linker = Linker::new(engine);

    wasmtime_wasi::p2::add_to_linker_sync(&mut linker)?;

    my_org::hardware::spi::add_to_linker::<HostState<T>, HasSelf<SpiImplementation<T>>>(
        &mut linker,
        |state: &mut HostState<T>| &mut state.spi_device,
    )?;

    Ok(linker)
}
