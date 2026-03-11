#![no_std]
extern crate alloc;

use alloc::boxed::Box;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;
use core::marker::PhantomData;
use wasmtime::component::{HasData, Linker, Resource, ResourceTable};

wasmtime::component::bindgen!({
    path: "../../wit/spi.wit",
    world: "wasi-spi-host",
    with: { "wasi:spi/spi.spi-device": ActiveSpiDriver }
});

use wasi::spi::spi;

pub struct ActiveSpiDriver {
    pub id: usize,
}

// ------------------------------------------------------------------
// 1. The Agnostic Trait (No configure!)
// ------------------------------------------------------------------

pub trait SpiHardware {
    fn read_data(&mut self, buf: &mut [u8]) -> Result<(), spi::Error>;
    fn write_data(&mut self, data: &[u8]) -> Result<(), spi::Error>;
    fn transfer_data(&mut self, rx: &mut [u8], tx: &[u8]) -> Result<(), spi::Error>;
}

// ------------------------------------------------------------------
// 2. The Universal `embedded-hal` Implementation
// ------------------------------------------------------------------
use embedded_hal::spi::SpiDevice;

// This implements SpiHardware for ANY standard SPI Device!
impl<T: SpiDevice> SpiHardware for T {
    fn read_data(&mut self, buf: &mut [u8]) -> Result<(), spi::Error> {
        self.read(buf)
            .map_err(|_| spi::Error::Other("Read failed".into()))
    }

    fn write_data(&mut self, data: &[u8]) -> Result<(), spi::Error> {
        self.write(data)
            .map_err(|_| spi::Error::Other("Write failed".into()))
    }

    fn transfer_data(&mut self, rx: &mut [u8], tx: &[u8]) -> Result<(), spi::Error> {
        self.transfer(rx, tx)
            .map_err(|_| spi::Error::Other("Transfer failed".into()))
    }
}

// ------------------------------------------------------------------
// 3. Wasmtime Host Context & Implementations
// ------------------------------------------------------------------

pub struct SpiCtx {
    pub table: ResourceTable,
    pub hardware: Vec<(String, Box<dyn SpiHardware + Send + 'static>)>,
}

pub trait SpiView {
    fn spi_ctx(&mut self) -> &mut SpiCtx;
}

pub struct SpiImpl<'a, T> {
    pub host: &'a mut T,
}

impl<'a, T: SpiView> SpiImpl<'a, T> {
    fn get_hw(
        &mut self,
        handle: &Resource<ActiveSpiDriver>,
    ) -> Result<&mut Box<dyn SpiHardware + Send + 'static>, spi::Error> {
        let id = self
            .host
            .spi_ctx()
            .table
            .get(handle)
            .map_err(|_| spi::Error::Other("Bad Handle".into()))?
            .id;

        self.host
            .spi_ctx()
            .hardware
            .get_mut(id)
            .map(|(_, hw)| hw)
            .ok_or_else(|| spi::Error::Other("HW unavailable".into()))
    }
}

impl<'a, T: SpiView> spi::Host for SpiImpl<'a, T> {
    fn get_devices(&mut self) -> Result<Vec<(String, Resource<ActiveSpiDriver>)>, spi::Error> {
        let mut devices = Vec::new();
        let ctx = self.host.spi_ctx();

        for (id, (name, _)) in ctx.hardware.iter().enumerate() {
            let handle = ctx
                .table
                .push(ActiveSpiDriver { id })
                .map_err(|e| spi::Error::Other(e.to_string()))?;
            devices.push((name.clone(), handle));
        }
        Ok(devices)
    }
}

impl<'a, T: SpiView> spi::HostSpiDevice for SpiImpl<'a, T> {
    fn read(&mut self, handle: Resource<ActiveSpiDriver>, len: u64) -> Result<Vec<u8>, spi::Error> {
        let mut buf = vec![0u8; len as usize];
        self.get_hw(&handle)?.read_data(&mut buf)?;
        Ok(buf)
    }

    fn write(
        &mut self,
        handle: Resource<ActiveSpiDriver>,
        data: Vec<u8>,
    ) -> Result<(), spi::Error> {
        self.get_hw(&handle)?.write_data(&data)
    }

    fn transfer(
        &mut self,
        handle: Resource<ActiveSpiDriver>,
        data: Vec<u8>,
    ) -> Result<Vec<u8>, spi::Error> {
        let mut rx = vec![0u8; data.len()];
        self.get_hw(&handle)?.transfer_data(&mut rx, &data)?;
        Ok(rx)
    }

    fn transaction(
        &mut self,
        _: Resource<ActiveSpiDriver>,
        _: Vec<spi::Operation>,
    ) -> Result<Vec<spi::OperationResult>, spi::Error> {
        Err(spi::Error::Other("Unsupported".into()))
    }

    fn drop(&mut self, rep: Resource<ActiveSpiDriver>) -> wasmtime::Result<()> {
        self.host.spi_ctx().table.delete(rep)?;
        Ok(())
    }
}

pub struct SpiBindingMarker<T>(PhantomData<T>);
impl<T: SpiView + 'static> HasData for SpiBindingMarker<T> {
    type Data<'a> = SpiImpl<'a, T>;
}
pub fn add_to_linker<T: SpiView + 'static>(linker: &mut Linker<T>) -> wasmtime::Result<()> {
    spi::add_to_linker::<T, SpiBindingMarker<T>>(linker, |host| SpiImpl { host })
}
