use __internal::Vec;
use embedded_hal::spi::SpiDevice as HalSpiDevice;
use wasmtime::component::{__internal, Resource};

pub mod bindings {
    use wasmtime::component::bindgen;
    bindgen!({
        path: "wit",
    });
}

use crate::bindings::my::hardware::spi::{Error, Host, HostSpiDevice, SpiDevice};

pub struct SpiContext<T> {
    pub bus: T,
}

impl<T> Host for SpiContext<T> where T: HalSpiDevice + Send + Sync + 'static {}

impl<T> HostSpiDevice for SpiContext<T>
where
    T: HalSpiDevice + Send + Sync + 'static,
    T::Error: std::fmt::Debug,
{
    fn read(&mut self, _self_: Resource<SpiDevice>, len: u64) -> Result<Vec<u8>, Error> {
        println!("SpiContext: read {} bytes", len);
        Err(Error::Busy)
    }

    fn write(&mut self, _self_: Resource<SpiDevice>, data: Vec<u8>) -> Result<(), Error> {
        println!("SpiContext: write {} bytes", data.len());
        Err(Error::Busy)
    }

    fn drop(&mut self, _rep: Resource<SpiDevice>) -> anyhow::Result<()> {
        println!("SpiContext: drop resource");
        Ok(())
    }
}
