use crate::ctx::{ActiveSpiDriver, WasiSpiCtx, WasiSpiView};
use crate::wasi::spi::spi as spi_bindings;
use embedded_hal::spi::{
    ErrorKind as HalErrorKind, Operation as HalOperation, SpiDevice as HalSpiDevice,
};
use linux_embedded_hal::SpidevDevice;
use wasmtime::component::{
    __internal::{String, Vec},
    Resource,
};
use wasmtime_wasi::ResourceTable;

pub struct SpiImpl<'a, T> {
    pub host: &'a mut T,
}

impl<'a, T: WasiSpiView> SpiImpl<'a, T> {
    fn ctx(&mut self) -> &mut WasiSpiCtx {
        self.host.spi_ctx()
    }

    fn table(&mut self) -> &mut ResourceTable {
        self.host.ctx().table
    }
}

fn from_hal_error<E: embedded_hal::spi::Error>(e: E) -> spi_bindings::Error {
    match e.kind() {
        HalErrorKind::Overrun => spi_bindings::Error::Overrun,
        HalErrorKind::ModeFault => spi_bindings::Error::ModeFault,
        HalErrorKind::FrameFormat => spi_bindings::Error::FrameFormat,
        HalErrorKind::ChipSelectFault => spi_bindings::Error::ChipSelectFault,
        HalErrorKind::Other => spi_bindings::Error::Other("Other HAL error".to_string()),
        // Handle any variants that don't map directly to 'Other' with the debug string
        _ => spi_bindings::Error::Other(format!("Unhandled HAL error: {:?}", e.kind())),
    }
}

enum TransactionBuffer {
    Read(Vec<u8>),
    Write(Vec<u8>),
    Transfer { read: Vec<u8>, write: Vec<u8> },
    Delay(u32),
}

impl TransactionBuffer {
    fn from_op(op: spi_bindings::Operation) -> Self {
        match op {
            spi_bindings::Operation::Read(len) => Self::Read(vec![0; len as usize]),
            spi_bindings::Operation::Write(data) => Self::Write(data),
            spi_bindings::Operation::Transfer(data) => Self::Transfer {
                read: vec![0; data.len()],
                write: data,
            },
            spi_bindings::Operation::DelayNs(ns) => Self::Delay(ns),
        }
    }

    fn as_hal_op(&mut self) -> HalOperation<'_, u8> {
        match self {
            Self::Read(buf) => HalOperation::Read(buf),
            Self::Write(buf) => HalOperation::Write(buf),
            Self::Transfer { read, write } => HalOperation::Transfer(read, write),
            Self::Delay(ns) => HalOperation::DelayNs(*ns),
        }
    }

    fn into_result(self) -> spi_bindings::OperationResult {
        match self {
            Self::Read(buf) => spi_bindings::OperationResult::Read(buf),
            Self::Write(_) => spi_bindings::OperationResult::Write, // In a write, the Guest should not be returned anything
            Self::Transfer { read, .. } => spi_bindings::OperationResult::Transfer(read),
            Self::Delay(_) => spi_bindings::OperationResult::Delay,
        }
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
            .ok_or_else(|| spi_bindings::Error::Other("Not found".to_string()))?;

        let physical = SpidevDevice::open(physical_path)
            .map_err(|e| spi_bindings::Error::Other(format!("Failed to open SPI device: {}", e)))?;

        let state = ActiveSpiDriver { device: physical };
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
        handle: Resource<ActiveSpiDriver>,
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
        handle: Resource<ActiveSpiDriver>,
        len: u64,
    ) -> Result<Vec<u8>, spi_bindings::Error> {
        let spi = self
            .table()
            .get_mut(&handle)
            .map_err(|e| spi_bindings::Error::Other(e.to_string()))?;

        let mut buf = vec![0u8; len as usize];

        spi.device.read(&mut buf).map_err(from_hal_error)?;

        Ok(buf)
    }

    fn write(
        &mut self,
        handle: Resource<ActiveSpiDriver>,
        data: Vec<u8>,
    ) -> Result<(), spi_bindings::Error> {
        let spi = self
            .table()
            .get_mut(&handle)
            .map_err(|e| spi_bindings::Error::Other(e.to_string()))?;

        spi.device.write(&data).map_err(from_hal_error)?;

        Ok(())
    }

    fn transfer(
        &mut self,
        handle: Resource<ActiveSpiDriver>,
        data: Vec<u8>,
    ) -> Result<Vec<u8>, spi_bindings::Error> {
        let spi = self
            .table()
            .get_mut(&handle)
            .map_err(|e| spi_bindings::Error::Other(e.to_string()))?;

        let mut read_buf = vec![0u8; data.len()];

        spi.device
            .transfer(&mut read_buf, &data)
            .map_err(from_hal_error)?;

        Ok(read_buf)
    }

    fn transaction(
        &mut self,
        handle: Resource<ActiveSpiDriver>,
        operations: Vec<spi_bindings::Operation>,
    ) -> Result<Vec<spi_bindings::OperationResult>, spi_bindings::Error> {
        let spi = self
            .table()
            .get_mut(&handle)
            .map_err(|e| spi_bindings::Error::Other(e.to_string()))?;

        let mut buffers: Vec<_> = operations
            .into_iter()
            .map(TransactionBuffer::from_op)
            .collect();

        let mut hal_ops: Vec<_> = buffers.iter_mut().map(|b| b.as_hal_op()).collect();
        spi.device
            .transaction(&mut hal_ops)
            .map_err(from_hal_error)?;

        Ok(buffers
            .into_iter()
            .map(TransactionBuffer::into_result)
            .collect())
    }

    fn drop(&mut self, rep: Resource<ActiveSpiDriver>) -> wasmtime::Result<()> {
        self.table().delete(rep)?;
        Ok(())
    }
}
