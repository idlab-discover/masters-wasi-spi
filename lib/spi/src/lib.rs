#![no_std]
extern crate alloc;

use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;
use core::marker::PhantomData;

use embassy_rp::gpio::Output;
use embassy_rp::peripherals::{SPI0, SPI1};
use embassy_rp::spi::{Blocking, Spi};
use wasmtime::component::{HasData, Linker, Resource, ResourceTable};

wasmtime::component::bindgen!({
    path: "../../wit/spi.wit",
    world: "wasi-spi-host",
    with: {
        "wasi:spi/spi.spi-device": ActiveSpiDriver
    }
});

pub struct ActiveSpiDriver {
    pub id: u8,
}

pub struct SpiCtx {
    pub table: ResourceTable,

    // SPI0 Hardware (BME280)
    pub spi0: Spi<'static, SPI0, Blocking>,
    pub cs0: Output<'static>,

    // SPI1 Hardware (OLED)
    pub spi1: Spi<'static, SPI1, Blocking>,
    pub cs1: Output<'static>,
}

pub trait SpiView {
    fn spi_ctx(&mut self) -> &mut SpiCtx;
}

pub struct SpiImpl<'a, T> {
    pub host: &'a mut T,
}

impl<'a, T: SpiView> wasi::spi::spi::Host for SpiImpl<'a, T> {
    fn get_device_names(&mut self) -> Vec<String> {
        vec!["spi0".to_string(), "spi1".to_string()]
    }

    fn open_device(
        &mut self,
        name: String,
    ) -> Result<Resource<ActiveSpiDriver>, wasi::spi::spi::Error> {
        if name == "spi0" {
            let handle = self
                .host
                .spi_ctx()
                .table
                .push(ActiveSpiDriver { id: 0 })
                .map_err(|e| wasi::spi::spi::Error::Other(e.to_string()))?;
            Ok(handle)
        } else if name == "spi1" {
            let handle = self
                .host
                .spi_ctx()
                .table
                .push(ActiveSpiDriver { id: 1 })
                .map_err(|e| wasi::spi::spi::Error::Other(e.to_string()))?;
            Ok(handle)
        } else {
            Err(wasi::spi::spi::Error::Other("Device not found".to_string()))
        }
    }
}

impl<'a, T: SpiView> wasi::spi::spi::HostSpiDevice for SpiImpl<'a, T> {
    fn configure(
        &mut self,
        _handle: Resource<ActiveSpiDriver>,
        _config: wasi::spi::spi::Config,
    ) -> Result<(), wasi::spi::spi::Error> {
        Ok(()) // Handled statically in main.rs for now
    }

    fn read(
        &mut self,
        handle: Resource<ActiveSpiDriver>,
        len: u64,
    ) -> Result<Vec<u8>, wasi::spi::spi::Error> {
        let mut buf = vec![0u8; len as usize];
        let driver = self
            .host
            .spi_ctx()
            .table
            .get(&handle)
            .map_err(|_| wasi::spi::spi::Error::Other("Invalid Handle".to_string()))?;

        match driver.id {
            0 => {
                self.host.spi_ctx().cs0.set_low();
                let res = self.host.spi_ctx().spi0.blocking_read(&mut buf);
                self.host.spi_ctx().cs0.set_high();
                res.map_err(|_| wasi::spi::spi::Error::Other("SPI0 read failed".to_string()))?;
            }
            1 => {
                self.host.spi_ctx().cs1.set_low();
                let res = self.host.spi_ctx().spi1.blocking_read(&mut buf);
                self.host.spi_ctx().cs1.set_high();
                res.map_err(|_| wasi::spi::spi::Error::Other("SPI1 read failed".to_string()))?;
            }
            _ => return Err(wasi::spi::spi::Error::Other("Unknown SPI ID".to_string())),
        }
        Ok(buf)
    }

    fn write(
        &mut self,
        handle: Resource<ActiveSpiDriver>,
        data: Vec<u8>,
    ) -> Result<(), wasi::spi::spi::Error> {
        let driver = self
            .host
            .spi_ctx()
            .table
            .get(&handle)
            .map_err(|_| wasi::spi::spi::Error::Other("Invalid Handle".to_string()))?;

        match driver.id {
            0 => {
                self.host.spi_ctx().cs0.set_low();
                let res = self.host.spi_ctx().spi0.blocking_write(&data);
                self.host.spi_ctx().cs0.set_high();
                res.map_err(|_| wasi::spi::spi::Error::Other("SPI0 write failed".to_string()))?;
            }
            1 => {
                self.host.spi_ctx().cs1.set_low();
                let res = self.host.spi_ctx().spi1.blocking_write(&data);
                self.host.spi_ctx().cs1.set_high();
                res.map_err(|_| wasi::spi::spi::Error::Other("SPI1 write failed".to_string()))?;
            }
            _ => return Err(wasi::spi::spi::Error::Other("Unknown SPI ID".to_string())),
        }
        Ok(())
    }

    fn transfer(
        &mut self,
        handle: Resource<ActiveSpiDriver>,
        data: Vec<u8>,
    ) -> Result<Vec<u8>, wasi::spi::spi::Error> {
        let mut read_buf = vec![0u8; data.len()];
        let driver = self
            .host
            .spi_ctx()
            .table
            .get(&handle)
            .map_err(|_| wasi::spi::spi::Error::Other("Invalid Handle".to_string()))?;

        match driver.id {
            0 => {
                self.host.spi_ctx().cs0.set_low();
                let res = self
                    .host
                    .spi_ctx()
                    .spi0
                    .blocking_transfer(&mut read_buf, &data);
                self.host.spi_ctx().cs0.set_high();
                res.map_err(|_| wasi::spi::spi::Error::Other("SPI0 transfer failed".to_string()))?;
            }
            1 => {
                self.host.spi_ctx().cs1.set_low();
                let res = self
                    .host
                    .spi_ctx()
                    .spi1
                    .blocking_transfer(&mut read_buf, &data);
                self.host.spi_ctx().cs1.set_high();
                res.map_err(|_| wasi::spi::spi::Error::Other("SPI1 transfer failed".to_string()))?;
            }
            _ => return Err(wasi::spi::spi::Error::Other("Unknown SPI ID".to_string())),
        }
        Ok(read_buf)
    }

    fn transaction(
        &mut self,
        _handle: Resource<ActiveSpiDriver>,
        _operations: Vec<wasi::spi::spi::Operation>,
    ) -> Result<Vec<wasi::spi::spi::OperationResult>, wasi::spi::spi::Error> {
        // You can duplicate the matching pattern above for transaction() as well if you use it in your code!
        Err(wasi::spi::spi::Error::Other(
            "Transaction unsupported".to_string(),
        ))
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
    wasi::spi::spi::add_to_linker::<T, SpiBindingMarker<T>>(linker, |host| SpiImpl { host })
}
