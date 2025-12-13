use embedded_hal::spi::{Operation as HalOperation, SpiDevice as HalSpiDevice};
use linux_embedded_hal::SpidevDevice;
use std::collections::HashMap;
use std::marker::PhantomData;
use wasmtime::component::{
    __internal::{String, Vec},
    HasData, Linker, Resource, bindgen,
};
use wasmtime_wasi::{ResourceTable, WasiView};

bindgen!({
    path: "../wit",
    world: "wasi-spi-host",
    with: {
        "wasi:spi/spi.spi-device": SpiDeviceState
    }
});

use wasi::spi::spi as spi_bindings;

pub struct Spi<T>(PhantomData<T>);

impl<T: WasiSpiView + 'static> HasData for Spi<T> {
    type Data<'a> = SpiImpl<'a, T>;
}

#[derive(Clone)]
pub struct SpiConfig {
    pub virtual_name: String,
    pub physical_path: String,
}
pub struct WasiSpiCtx {
    pub devices: HashMap<String, String>,
}

impl WasiSpiCtx {
    pub fn from_configs(configs: Vec<SpiConfig>) -> anyhow::Result<Self> {
        let mut devices = HashMap::new();

        for config in configs {
            devices.insert(config.virtual_name, config.physical_path);
        }

        Ok(Self { devices: devices })
    }
}

pub struct SpiDeviceState {
    pub device: SpidevDevice,
}

pub trait WasiSpiView: WasiView {
    fn spi_ctx(&mut self) -> &mut WasiSpiCtx;
}

pub fn add_to_linker<T>(linker: &mut Linker<T>) -> anyhow::Result<()>
where
    T: WasiSpiView + 'static,
{
    spi_bindings::add_to_linker::<T, Spi<T>>(linker, |host| SpiImpl { host })
}

pub struct SpiImpl<'a, T> {
    host: &'a mut T,
}

impl<'a, T: WasiSpiView> SpiImpl<'a, T> {
    fn ctx(&mut self) -> &mut WasiSpiCtx {
        self.host.spi_ctx()
    }

    fn table(&mut self) -> &mut ResourceTable {
        self.host.ctx().table
    }
}

impl<'a, T: WasiSpiView> spi_bindings::Host for SpiImpl<'a, T> {
    fn get_device_names(&mut self) -> Vec<String> {
        self.ctx().devices.keys().cloned().collect()
    }

    fn open_device(
        &mut self,
        name: String,
    ) -> Result<Resource<spi_bindings::SpiDevice>, spi_bindings::Error> {
        let physical_path = self
            .ctx()
            .devices
            .get(&name)
            .ok_or(spi_bindings::Error::Other("Not found".to_string()))?;

        let physical = SpidevDevice::open(physical_path)
            .map_err(|e| spi_bindings::Error::Other(format!("Failed to open SPI device: {}", e)))?;

        let state = SpiDeviceState { device: physical };
        let handle = self
            .table()
            .push(state)
            .map_err(|e| spi_bindings::Error::Other(e.to_string()))?;

        Ok(handle)
    }
}

impl<'a, T: WasiSpiView> spi_bindings::HostSpiDevice for SpiImpl<'a, T> {
    fn configure(
        &mut self,
        handle: Resource<SpiDeviceState>,
        config: spi_bindings::Config,
    ) -> Result<(), spi_bindings::Error> {
        let spi = self
            .table()
            .get_mut(&handle)
            .map_err(|e| spi_bindings::Error::Other(e.to_string()))?;

        use linux_embedded_hal::spidev::{SpiModeFlags, SpidevOptions};

        let mode = match config.mode {
            spi_bindings::Mode::Mode0 => SpiModeFlags::SPI_MODE_0,
            spi_bindings::Mode::Mode1 => SpiModeFlags::SPI_MODE_1,
            spi_bindings::Mode::Mode2 => SpiModeFlags::SPI_MODE_2,
            spi_bindings::Mode::Mode3 => SpiModeFlags::SPI_MODE_3,
        };

        let options = SpidevOptions::new()
            .max_speed_hz(config.frequency)
            .mode(mode)
            .lsb_first(config.lsb_first)
            .build();

        spi.device
            .configure(&options)
            .map_err(|e| spi_bindings::Error::Other(e.to_string()))?;

        Ok(())
    }

    fn read(
        &mut self,
        handle: Resource<SpiDeviceState>,
        len: u64,
    ) -> Result<Vec<u8>, spi_bindings::Error> {
        let spi = self
            .table()
            .get_mut(&handle)
            .map_err(|e| spi_bindings::Error::Other(e.to_string()))?;

        let mut buf = vec![0u8; len as usize];

        spi.device
            .read(&mut buf)
            .map_err(|e| spi_bindings::Error::Other(e.to_string()))?;

        Ok(buf)
    }

    fn write(
        &mut self,
        handle: Resource<SpiDeviceState>,
        data: Vec<u8>,
    ) -> Result<(), spi_bindings::Error> {
        let spi = self
            .table()
            .get_mut(&handle)
            .map_err(|e| spi_bindings::Error::Other(e.to_string()))?;

        spi.device
            .write(&data)
            .map_err(|e| spi_bindings::Error::Other(e.to_string()))?;

        Ok(())
    }

    fn transfer(
        &mut self,
        handle: Resource<SpiDeviceState>,
        data: Vec<u8>,
    ) -> Result<Vec<u8>, spi_bindings::Error> {
        let spi = self
            .table()
            .get_mut(&handle)
            .map_err(|e| spi_bindings::Error::Other(e.to_string()))?;

        let mut read_buf = vec![0u8; data.len()];

        spi.device
            .transfer(&mut read_buf, &data)
            .map_err(|e| spi_bindings::Error::Other(e.to_string()))?;

        Ok(read_buf)
    }

    fn transaction(
        &mut self,
        handle: Resource<SpiDeviceState>,
        operations: Vec<spi_bindings::Operation>,
    ) -> Result<Vec<spi_bindings::OperationResult>, spi_bindings::Error> {
        let spi = self
            .table()
            .get_mut(&handle)
            .map_err(|e| spi_bindings::Error::Other(e.to_string()))?;

        enum TransactionBuffers {
            Read(Vec<u8>),
            Write(Vec<u8>),
            Transfer { read: Vec<u8>, write: Vec<u8> },
            DelayNs(u32),
        }

        let mut buffers: Vec<TransactionBuffers> = operations
            .into_iter()
            .map(|operation| match operation {
                spi_bindings::Operation::Read(len) => {
                    TransactionBuffers::Read(vec![0u8; len as usize])
                }
                spi_bindings::Operation::Write(write_buf) => TransactionBuffers::Write(write_buf),
                spi_bindings::Operation::Transfer(write_buf) => TransactionBuffers::Transfer {
                    read: vec![0u8; write_buf.len()],
                    write: write_buf,
                },
                spi_bindings::Operation::DelayNs(ns) => TransactionBuffers::DelayNs(ns),
            })
            .collect();

        let mut hal_operations: Vec<HalOperation<u8>> = buffers
            .iter_mut()
            .map(|buffer| match buffer {
                TransactionBuffers::Read(buf) => HalOperation::Read(buf),
                TransactionBuffers::Write(buf) => HalOperation::Write(buf),
                TransactionBuffers::Transfer { read, write } => HalOperation::Transfer(read, write),
                TransactionBuffers::DelayNs(ns) => HalOperation::DelayNs(*ns),
            })
            .collect();

        spi.device
            .transaction(&mut hal_operations)
            .map_err(|e| spi_bindings::Error::Other(e.to_string()))?;

        let results = buffers
            .into_iter()
            .map(|buffer| match buffer {
                TransactionBuffers::Read(data) => spi_bindings::OperationResult::Read(data),
                TransactionBuffers::Write(_) => spi_bindings::OperationResult::Write,
                TransactionBuffers::Transfer { read, .. } => {
                    spi_bindings::OperationResult::Transfer(read)
                }
                TransactionBuffers::DelayNs(_) => spi_bindings::OperationResult::Delay,
            })
            .collect();

        Ok(results)
    }

    fn drop(&mut self, rep: Resource<SpiDeviceState>) -> wasmtime::Result<()> {
        self.table().delete(rep)?;
        Ok(())
    }
}
