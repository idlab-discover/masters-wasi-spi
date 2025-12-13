use wasmtime::component::Linker;
use wasmtime::component::bindgen;

pub mod ctx;
pub mod impls;

pub use ctx::{Spi, SpiConfig, WasiSpiCtx, WasiSpiView};
use impls::SpiImpl;

bindgen!({
    path: "../wit",
    world: "wasi-spi-host",
    with: {
        "wasi:spi/spi.spi-device": ctx::SpiDeviceState
    }
});

pub fn add_to_linker<T>(linker: &mut Linker<T>) -> anyhow::Result<()>
where
    T: WasiSpiView + 'static,
{
    wasi::spi::spi::add_to_linker::<T, Spi<T>>(linker, |host| SpiImpl { host })
}
