use __internal::Vec;
use embedded_hal::spi::SpiBus;
use wasmtime::component::{bindgen, Resource, __internal};


bindgen!({
    path: "wit",
});

use crate::my::hardware::spi::{Error, HostSpiDevice, SpiDevice};

pub struct SpiContext<T> {
    pub bus: T,
}

impl<T> HostSpiDevice for SpiContext<T>
where
    T: SpiBus + Send + Sync + 'static,
    T::Error: std::fmt::Debug,
{
    fn read(&mut self, self_: Resource<SpiDevice>, len: u64) -> Result<Vec<u8>, Error> {
        println!("SpiContext: read {} bytes", len);
        Err(Error::Busy)
    }

    fn write(&mut self, self_: Resource<SpiDevice>, data: Vec<u8>) -> Result<(), Error> {
        println!("SpiContext: write {} bytes", data.len());
        Err(Error::Busy)
    }

    fn transfer(&mut self, self_: Resource<SpiDevice>, data: Vec<u8>) -> Result<Vec<u8>, Error> {
        println!("SpiContext: transfer {} bytes", data.len());
        Err(Error::Busy)
    }

    fn drop(&mut self, rep: Resource<SpiDevice>) -> anyhow::Result<()> {
        println!("SpiContext: drop resource");
        Ok(())
    }
}