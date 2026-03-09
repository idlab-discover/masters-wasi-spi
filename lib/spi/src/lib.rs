#![no_std]
extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;
use core::marker::PhantomData;

use embassy_rp::gpio::Output;
use embassy_rp::peripherals::{SPI0, SPI1};
use embassy_rp::spi::{Blocking, Config as RpSpiConfig, Phase, Polarity, Spi};
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

    // Maps policy names (e.g., "sensor") to an internal hardware ID (0 for SPI0, 1 for SPI1)
    pub device_map: BTreeMap<String, u8>,

    // Hardware is wrapped in Option so it can be safely omitted if not defined in policy.toml
    pub spi0: Option<(Spi<'static, SPI0, Blocking>, Output<'static>)>,
    pub spi1: Option<(Spi<'static, SPI1, Blocking>, Output<'static>)>,
}

pub trait SpiView {
    fn spi_ctx(&mut self) -> &mut SpiCtx;
}

pub struct SpiImpl<'a, T> {
    pub host: &'a mut T,
}

impl<'a, T: SpiView> wasi::spi::spi::Host for SpiImpl<'a, T> {
    fn get_device_names(&mut self) -> Vec<String> {
        // Only return the names explicitly mapped in the policy.toml
        self.host.spi_ctx().device_map.keys().cloned().collect()
    }

    fn open_device(
        &mut self,
        name: String,
    ) -> Result<Resource<ActiveSpiDriver>, wasi::spi::spi::Error> {
        // Strict lookup: if it's not in the map, the guest cannot open it
        if let Some(&id) = self.host.spi_ctx().device_map.get(&name) {
            let handle = self
                .host
                .spi_ctx()
                .table
                .push(ActiveSpiDriver { id })
                .map_err(|e| wasi::spi::spi::Error::Other(e.to_string()))?;
            Ok(handle)
        } else {
            Err(wasi::spi::spi::Error::Other(
                "Device not found or not mapped in policy".to_string(),
            ))
        }
    }
}

impl<'a, T: SpiView> wasi::spi::spi::HostSpiDevice for SpiImpl<'a, T> {
    fn configure(
        &mut self,
        handle: Resource<ActiveSpiDriver>,
        config: wasi::spi::spi::Config,
    ) -> Result<(), wasi::spi::spi::Error> {
        let driver = self
            .host
            .spi_ctx()
            .table
            .get(&handle)
            .map_err(|_| wasi::spi::spi::Error::Other("Invalid Handle".to_string()))?;

        // 1. Setup the Embassy SPI configuration
        let mut rp_config = RpSpiConfig::default();
        rp_config.frequency = config.frequency;

        let (polarity, phase) = match config.mode {
            wasi::spi::spi::Mode::Mode0 => (Polarity::IdleLow, Phase::CaptureOnFirstTransition),
            wasi::spi::spi::Mode::Mode1 => (Polarity::IdleLow, Phase::CaptureOnSecondTransition),
            wasi::spi::spi::Mode::Mode2 => (Polarity::IdleHigh, Phase::CaptureOnFirstTransition),
            wasi::spi::spi::Mode::Mode3 => (Polarity::IdleHigh, Phase::CaptureOnSecondTransition),
        };

        rp_config.polarity = polarity;
        rp_config.phase = phase;

        if config.lsb_first {
            return Err(wasi::spi::spi::Error::Other(
                "LSB-first natively unsupported by host".to_string(),
            ));
        }

        // 2. Apply it directly to the targeted physical SPI block
        match driver.id {
            0 => {
                let (spi, _) = self.host.spi_ctx().spi0.as_mut().ok_or_else(|| {
                    wasi::spi::spi::Error::Other("SPI0 not enabled in policy".to_string())
                })?;
                spi.set_config(&rp_config);
            }
            1 => {
                let (spi, _) = self.host.spi_ctx().spi1.as_mut().ok_or_else(|| {
                    wasi::spi::spi::Error::Other("SPI1 not enabled in policy".to_string())
                })?;
                spi.set_config(&rp_config);
            }
            _ => return Err(wasi::spi::spi::Error::Other("Unknown SPI ID".to_string())),
        }

        Ok(())
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
                let (spi, cs) = self.host.spi_ctx().spi0.as_mut().ok_or_else(|| {
                    wasi::spi::spi::Error::Other("SPI0 not enabled in policy".to_string())
                })?;
                cs.set_low();
                let res = spi.blocking_read(&mut buf);
                cs.set_high();
                res.map_err(|_| wasi::spi::spi::Error::Other("SPI0 read failed".to_string()))?;
            }
            1 => {
                let (spi, cs) = self.host.spi_ctx().spi1.as_mut().ok_or_else(|| {
                    wasi::spi::spi::Error::Other("SPI1 not enabled in policy".to_string())
                })?;
                cs.set_low();
                let res = spi.blocking_read(&mut buf);
                cs.set_high();
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
                let (spi, cs) = self.host.spi_ctx().spi0.as_mut().ok_or_else(|| {
                    wasi::spi::spi::Error::Other("SPI0 not enabled in policy".to_string())
                })?;
                cs.set_low();
                let res = spi.blocking_write(&data);
                cs.set_high();
                res.map_err(|_| wasi::spi::spi::Error::Other("SPI0 write failed".to_string()))?;
            }
            1 => {
                let (spi, cs) = self.host.spi_ctx().spi1.as_mut().ok_or_else(|| {
                    wasi::spi::spi::Error::Other("SPI1 not enabled in policy".to_string())
                })?;
                cs.set_low();
                let res = spi.blocking_write(&data);
                cs.set_high();
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
                let (spi, cs) = self.host.spi_ctx().spi0.as_mut().ok_or_else(|| {
                    wasi::spi::spi::Error::Other("SPI0 not enabled in policy".to_string())
                })?;
                cs.set_low();
                let res = spi.blocking_transfer(&mut read_buf, &data);
                cs.set_high();
                res.map_err(|_| wasi::spi::spi::Error::Other("SPI0 transfer failed".to_string()))?;
            }
            1 => {
                let (spi, cs) = self.host.spi_ctx().spi1.as_mut().ok_or_else(|| {
                    wasi::spi::spi::Error::Other("SPI1 not enabled in policy".to_string())
                })?;
                cs.set_low();
                let res = spi.blocking_transfer(&mut read_buf, &data);
                cs.set_high();
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
