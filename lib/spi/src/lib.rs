#![no_std]
extern crate alloc;

use alloc::boxed::Box;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;
use core::marker::PhantomData;

use embassy_rp::gpio::Output;
use embassy_rp::spi::{Blocking, Config as RpSpiConfig, Instance, Phase, Polarity, Spi};
use wasmtime::component::{HasData, Linker, Resource, ResourceTable};

wasmtime::component::bindgen!({
    path: "../../wit/spi.wit",
    world: "wasi-spi-host",
    with: { "wasi:spi/spi.spi-device": ActiveSpiDriver }
});

pub struct ActiveSpiDriver {
    pub id: usize, // Now just an array index!
}

pub trait SpiHardware {
    fn configure(&mut self, config: &RpSpiConfig) -> Result<(), wasi::spi::spi::Error>;
    fn read_data(&mut self, buf: &mut [u8]) -> Result<(), wasi::spi::spi::Error>;
    fn write_data(&mut self, data: &[u8]) -> Result<(), wasi::spi::spi::Error>;
    fn transfer_data(&mut self, rx: &mut [u8], tx: &[u8]) -> Result<(), wasi::spi::spi::Error>;
}

// 1. Embassy Implementation remains exactly the same
impl<'d, T: Instance> SpiHardware for (Spi<'d, T, Blocking>, Output<'d>) {
    fn configure(&mut self, config: &RpSpiConfig) -> Result<(), wasi::spi::spi::Error> {
        self.0.set_config(config);
        Ok(())
    }
    fn read_data(&mut self, buf: &mut [u8]) -> Result<(), wasi::spi::spi::Error> {
        self.1.set_low();
        let res = self.0.blocking_read(buf);
        self.1.set_high();
        res.map_err(|_| wasi::spi::spi::Error::Other("Read failed".into()))
    }
    fn write_data(&mut self, data: &[u8]) -> Result<(), wasi::spi::spi::Error> {
        self.1.set_low();
        let res = self.0.blocking_write(data);
        self.1.set_high();
        res.map_err(|_| wasi::spi::spi::Error::Other("Write failed".into()))
    }
    fn transfer_data(&mut self, rx: &mut [u8], tx: &[u8]) -> Result<(), wasi::spi::spi::Error> {
        self.1.set_low();
        let res = self.0.blocking_transfer(rx, tx);
        self.1.set_high();
        res.map_err(|_| wasi::spi::spi::Error::Other("Transfer failed".into()))
    }
}

// 2. Dramatically Simplified Context
pub struct SpiCtx {
    pub table: ResourceTable,
    pub hardware: Vec<(String, Box<dyn SpiHardware + Send + 'static>)>, // Unified list
}

pub trait SpiView {
    fn spi_ctx(&mut self) -> &mut SpiCtx;
}

pub struct SpiImpl<'a, T> {
    pub host: &'a mut T,
}

impl<'a, T: SpiView> SpiImpl<'a, T> {
    // 3. Lightning fast O(1) array lookup
    fn get_hw(
        &mut self,
        handle: &Resource<ActiveSpiDriver>,
    ) -> Result<&mut Box<dyn SpiHardware + Send + 'static>, wasi::spi::spi::Error> {
        let id = self
            .host
            .spi_ctx()
            .table
            .get(handle)
            .map_err(|_| wasi::spi::spi::Error::Other("Bad Handle".into()))?
            .id;

        self.host
            .spi_ctx()
            .hardware
            .get_mut(id)
            .map(|(_, hw)| hw) // Just pass back the &mut Box
            .ok_or_else(|| wasi::spi::spi::Error::Other("HW unavailable".into()))
    }
}

impl<'a, T: SpiView> wasi::spi::spi::Host for SpiImpl<'a, T> {
    fn get_devices(
        &mut self,
    ) -> Result<Vec<(String, Resource<ActiveSpiDriver>)>, wasi::spi::spi::Error> {
        let mut devices = Vec::new();
        let ctx = self.host.spi_ctx();

        // 4. Disjoint borrowing perfectly splits the table and the hardware list
        for (id, (name, _)) in ctx.hardware.iter().enumerate() {
            let handle = ctx
                .table
                .push(ActiveSpiDriver { id })
                .map_err(|e| wasi::spi::spi::Error::Other(e.to_string()))?;
            devices.push((name.clone(), handle));
        }
        Ok(devices)
    }
}

impl<'a, T: SpiView> wasi::spi::spi::HostSpiDevice for SpiImpl<'a, T> {
    fn configure(
        &mut self,
        handle: Resource<ActiveSpiDriver>,
        config: wasi::spi::spi::Config,
    ) -> Result<(), wasi::spi::spi::Error> {
        if config.lsb_first {
            return Err(wasi::spi::spi::Error::Other(
                "LSB natively unsupported".into(),
            ));
        }

        let mut rp_config = RpSpiConfig::default();
        rp_config.frequency = config.frequency;
        (rp_config.polarity, rp_config.phase) = match config.mode {
            wasi::spi::spi::Mode::Mode0 => (Polarity::IdleLow, Phase::CaptureOnFirstTransition),
            wasi::spi::spi::Mode::Mode1 => (Polarity::IdleLow, Phase::CaptureOnSecondTransition),
            wasi::spi::spi::Mode::Mode2 => (Polarity::IdleHigh, Phase::CaptureOnFirstTransition),
            wasi::spi::spi::Mode::Mode3 => (Polarity::IdleHigh, Phase::CaptureOnSecondTransition),
        };
        self.get_hw(&handle)?.configure(&rp_config)
    }

    fn read(
        &mut self,
        handle: Resource<ActiveSpiDriver>,
        len: u64,
    ) -> Result<Vec<u8>, wasi::spi::spi::Error> {
        let mut buf = vec![0u8; len as usize];
        self.get_hw(&handle)?.read_data(&mut buf)?;
        Ok(buf)
    }

    fn write(
        &mut self,
        handle: Resource<ActiveSpiDriver>,
        data: Vec<u8>,
    ) -> Result<(), wasi::spi::spi::Error> {
        self.get_hw(&handle)?.write_data(&data)
    }

    fn transfer(
        &mut self,
        handle: Resource<ActiveSpiDriver>,
        data: Vec<u8>,
    ) -> Result<Vec<u8>, wasi::spi::spi::Error> {
        let mut rx = vec![0u8; data.len()];
        self.get_hw(&handle)?.transfer_data(&mut rx, &data)?;
        Ok(rx)
    }

    fn transaction(
        &mut self,
        _: Resource<ActiveSpiDriver>,
        _: Vec<wasi::spi::spi::Operation>,
    ) -> Result<Vec<wasi::spi::spi::OperationResult>, wasi::spi::spi::Error> {
        Err(wasi::spi::spi::Error::Other("Unsupported".into()))
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
