use anyhow::Context;
use embedded_hal::spi::{Operation as HalOperation, SpiDevice as HalSpiDevice};
use linux_embedded_hal::SpidevDevice;
use std::collections::HashMap;
use std::marker::PhantomData;
use wasmtime::component::{HasData, Linker, Resource, bindgen};
use wasmtime_wasi::{ResourceTable, WasiView};

bindgen!({
    path: "../wit",
    world: "wasi-spi-host",
    with: {
        "wasi:spi/spi.spi-device": SpiDeviceState
    }
});

pub struct Spi<T>(PhantomData<T>);

impl<T: WasiSpiView + 'static> HasData for Spi<T> {
    type Data<'a> = SpiImpl<'a, T>;
}

pub struct SpiConfig {
    pub virtual_name: String,
    pub physical_path: String,
}
pub struct WasiSpiCtx {
    pub devices: HashMap<String, Resource<SpiDeviceState>>,
}

impl WasiSpiCtx {
    pub fn from_configs(
        table: &mut ResourceTable,
        configs: Vec<SpiConfig>,
    ) -> anyhow::Result<Self> {
        let mut devices = HashMap::new();

        for config in configs {
            // Open the physical device
            let physical = SpidevDevice::open(&config.physical_path).with_context(|| {
                format!("Failed to open SPI device at '{}'", config.physical_path)
            })?;

            // Wrap in state struct
            let state = SpiDeviceState { device: physical };

            // Push to table to get the Resource handle
            let handle = table.push(state)?;

            // Store in our map
            devices.insert(config.virtual_name, handle);
        }

        Ok(Self { devices })
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
    wasi::spi::spi::add_to_linker::<T, Spi<T>>(linker, |host| SpiImpl { host })
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

impl<'a, T: WasiSpiView> wasi::spi::spi::Host for SpiImpl<'a, T> {
    fn get_devices(&mut self) -> Vec<wasi::spi::spi::NamedDevice> {
        self.ctx()
            .devices
            .iter()
            .map(|(name, resource)| wasi::spi::spi::NamedDevice {
                name: name.clone(),
                device: Resource::new_own(resource.rep()),
            })
            .collect()
    }
}

impl<'a, T: WasiSpiView> wasi::spi::spi::HostSpiDevice for SpiImpl<'a, T> {
    fn configure(
        &mut self,
        handle: Resource<SpiDeviceState>,
        config: wasi::spi::spi::Config,
    ) -> Result<(), wasi::spi::spi::Error> {
        let spi = self
            .table()
            .get_mut(&handle)
            .map_err(|e| wasi::spi::spi::Error::Other(e.to_string()))?;

        use linux_embedded_hal::spidev::{SpiModeFlags, SpidevOptions};

        let mode = match config.mode {
            wasi::spi::spi::Mode::Mode0 => SpiModeFlags::SPI_MODE_0,
            wasi::spi::spi::Mode::Mode1 => SpiModeFlags::SPI_MODE_1,
            wasi::spi::spi::Mode::Mode2 => SpiModeFlags::SPI_MODE_2,
            wasi::spi::spi::Mode::Mode3 => SpiModeFlags::SPI_MODE_3,
        };

        let options = SpidevOptions::new()
            .max_speed_hz(config.frequency)
            .mode(mode)
            .lsb_first(config.lsb_first)
            .build();

        spi.device
            .configure(&options)
            .map_err(|e| wasi::spi::spi::Error::Other(e.to_string()))?;

        Ok(())
    }

    fn read(
        &mut self,
        handle: Resource<SpiDeviceState>,
        len: u64,
    ) -> Result<Vec<u8>, wasi::spi::spi::Error> {
        let spi = self
            .table()
            .get_mut(&handle)
            .map_err(|e| wasi::spi::spi::Error::Other(e.to_string()))?;

        let mut buf = vec![0u8; len as usize];

        spi.device
            .read(&mut buf)
            .map_err(|e| wasi::spi::spi::Error::Other(e.to_string()))?;

        Ok(buf)
    }

    fn write(
        &mut self,
        handle: Resource<SpiDeviceState>,
        data: Vec<u8>,
    ) -> Result<(), wasi::spi::spi::Error> {
        let spi = self
            .table()
            .get_mut(&handle)
            .map_err(|e| wasi::spi::spi::Error::Other(e.to_string()))?;

        spi.device
            .write(&data)
            .map_err(|e| wasi::spi::spi::Error::Other(e.to_string()))?;

        Ok(())
    }

    fn transfer(
        &mut self,
        handle: Resource<SpiDeviceState>,
        data: Vec<u8>,
    ) -> Result<Vec<u8>, wasi::spi::spi::Error> {
        let spi = self
            .table()
            .get_mut(&handle)
            .map_err(|e| wasi::spi::spi::Error::Other(e.to_string()))?;

        let mut read_buf = vec![0u8; data.len()];

        spi.device
            .transfer(&mut read_buf, &data)
            .map_err(|e| wasi::spi::spi::Error::Other(e.to_string()))?;

        Ok(read_buf)
    }

    fn transaction(
        &mut self,
        handle: Resource<SpiDeviceState>,
        operations: Vec<wasi::spi::spi::Operation>,
    ) -> Result<Vec<wasi::spi::spi::OperationResult>, wasi::spi::spi::Error> {
        let spi = self
            .table()
            .get_mut(&handle)
            .map_err(|e| wasi::spi::spi::Error::Other(e.to_string()))?;

        enum TransactionBuffers {
            Read(Vec<u8>),
            Write(Vec<u8>),
            Transfer { read: Vec<u8>, write: Vec<u8> },
            DelayNs(u32),
        }

        let mut buffers: Vec<TransactionBuffers> = operations
            .into_iter()
            .map(|operation| match operation {
                wasi::spi::spi::Operation::Read(len) => {
                    TransactionBuffers::Read(vec![0u8; len as usize])
                }
                wasi::spi::spi::Operation::Write(write_buf) => TransactionBuffers::Write(write_buf),
                wasi::spi::spi::Operation::Transfer(write_buf) => TransactionBuffers::Transfer {
                    read: vec![0u8; write_buf.len()],
                    write: write_buf,
                },
                wasi::spi::spi::Operation::DelayNs(ns) => TransactionBuffers::DelayNs(ns),
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
            .map_err(|e| wasi::spi::spi::Error::Other(e.to_string()))?;

        let results = buffers
            .into_iter()
            .map(|buffer| match buffer {
                TransactionBuffers::Read(data) => wasi::spi::spi::OperationResult::Read(data),
                TransactionBuffers::Write(_) => wasi::spi::spi::OperationResult::Write,
                TransactionBuffers::Transfer { read, .. } => {
                    wasi::spi::spi::OperationResult::Transfer(read)
                }
                TransactionBuffers::DelayNs(_) => wasi::spi::spi::OperationResult::Delay,
            })
            .collect();

        Ok(results)
    }

    fn drop(&mut self, rep: Resource<SpiDeviceState>) -> wasmtime::Result<()> {
        let rep_id = rep.rep();
        self.table().delete(rep)?;
        self.ctx().devices.retain(|_, r| r.rep() != rep_id);
        Ok(())
    }
}
