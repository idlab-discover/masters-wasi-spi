#![no_std]
extern crate alloc;

use alloc::boxed::Box;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;
use embedded_hal::spi::{Error as HalError, ErrorKind, Operation as HalOperation, SpiDevice};
use wasmtime::component::{Linker, Resource, ResourceTable};

wasmtime::component::bindgen!({
    path: "../../../wit/spi.wit",
    world: "wasi-spi-host",
    with: { "wasi:spi/spi.spi-device": ActiveSpiDriver }
});

use wasi::spi::spi;

pub struct ActiveSpiDriver {
    pub id: usize,
}

pub trait ErasedSpiDevice {
    fn read(&mut self, buf: &mut [u8]) -> Result<(), spi::Error>;
    fn write(&mut self, data: &[u8]) -> Result<(), spi::Error>;
    fn transfer(&mut self, rx: &mut [u8], tx: &[u8]) -> Result<(), spi::Error>;
    fn transaction(&mut self, operations: &mut [HalOperation<'_, u8>]) -> Result<(), spi::Error>;
}

fn map_hal_error<E: HalError>(err: E) -> spi::Error {
    match err.kind() {
        ErrorKind::Overrun => spi::Error::Overrun,
        ErrorKind::ModeFault => spi::Error::ModeFault,
        ErrorKind::FrameFormat => spi::Error::FrameFormat,
        ErrorKind::ChipSelectFault => spi::Error::ChipSelectFault,
        _ => spi::Error::Other("Hardware SPI error".to_string()),
    }
}

impl<T: SpiDevice<u8>> ErasedSpiDevice for T {
    fn read(&mut self, buf: &mut [u8]) -> Result<(), spi::Error> {
        SpiDevice::read(self, buf).map_err(map_hal_error)
    }

    fn write(&mut self, data: &[u8]) -> Result<(), spi::Error> {
        SpiDevice::write(self, data).map_err(map_hal_error)
    }

    fn transfer(&mut self, rx: &mut [u8], tx: &[u8]) -> Result<(), spi::Error> {
        SpiDevice::transfer(self, rx, tx).map_err(map_hal_error)
    }

    fn transaction(&mut self, operations: &mut [HalOperation<'_, u8>]) -> Result<(), spi::Error> {
        SpiDevice::transaction(self, operations).map_err(map_hal_error)
    }
}

enum TransactionBuffer {
    Read(Vec<u8>),
    Write(Vec<u8>),
    Transfer { read: Vec<u8>, write: Vec<u8> },
    Delay(u32),
}

impl TransactionBuffer {
    fn from_op(op: spi::Operation) -> Self {
        match op {
            spi::Operation::Read(len) => Self::Read(vec![0; len as usize]),
            spi::Operation::Write(data) => Self::Write(data),
            spi::Operation::Transfer(data) => Self::Transfer {
                read: vec![0; data.len()],
                write: data,
            },
            spi::Operation::DelayNs(ns) => Self::Delay(ns),
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

    fn into_result(self) -> spi::OperationResult {
        match self {
            Self::Read(buf) => spi::OperationResult::Read(buf),
            Self::Write(_) => spi::OperationResult::Write,
            Self::Transfer { read, .. } => spi::OperationResult::Transfer(read),
            Self::Delay(_) => spi::OperationResult::Delay,
        }
    }
}

pub struct SpiCtx {
    pub table: ResourceTable,
    pub hardware: Vec<(String, Box<dyn ErasedSpiDevice + Send + 'static>)>,
}

pub trait SpiView {
    fn spi_ctx(&mut self) -> &mut SpiCtx;
}

impl SpiCtx {
    fn get_hw(
        &mut self,
        handle: &Resource<ActiveSpiDriver>,
    ) -> Result<&mut Box<dyn ErasedSpiDevice + Send + 'static>, spi::Error> {
        let id = self
            .table
            .get(handle)
            .map_err(|_| spi::Error::Other("Bad Handle".into()))?
            .id;

        self.hardware
            .get_mut(id)
            .map(|(_, hw)| hw)
            .ok_or_else(|| spi::Error::Other("HW unavailable".into()))
    }
}

impl spi::Host for SpiCtx {
    fn open(&mut self, name: String) -> Result<Resource<ActiveSpiDriver>, spi::Error> {
        let (id, _) = self
            .hardware
            .iter()
            .enumerate()
            .find(|(_, (hw_name, _))| hw_name == &name)
            .ok_or_else(|| spi::Error::Other(alloc::format!("Device '{}' not found", name)))?;

        self.table
            .push(ActiveSpiDriver { id })
            .map_err(|e| spi::Error::Other(e.to_string()))
    }
}

impl spi::HostSpiDevice for SpiCtx {
    fn read(&mut self, handle: Resource<ActiveSpiDriver>, len: u64) -> Result<Vec<u8>, spi::Error> {
        let mut buf = vec![0u8; len as usize];
        self.get_hw(&handle)?.read(&mut buf)?;
        Ok(buf)
    }

    fn write(
        &mut self,
        handle: Resource<ActiveSpiDriver>,
        data: Vec<u8>,
    ) -> Result<(), spi::Error> {
        self.get_hw(&handle)?.write(&data)
    }

    fn transfer(
        &mut self,
        handle: Resource<ActiveSpiDriver>,
        data: Vec<u8>,
    ) -> Result<Vec<u8>, spi::Error> {
        let mut rx = vec![0u8; data.len()];
        self.get_hw(&handle)?.transfer(&mut rx, &data)?;
        Ok(rx)
    }

    fn transaction(
        &mut self,
        handle: Resource<ActiveSpiDriver>,
        operations: Vec<spi::Operation>,
    ) -> Result<Vec<spi::OperationResult>, spi::Error> {
        let hw = self.get_hw(&handle)?;

        let mut buffers: Vec<_> = operations
            .into_iter()
            .map(TransactionBuffer::from_op)
            .collect();

        let mut hal_ops: Vec<_> = buffers.iter_mut().map(|b| b.as_hal_op()).collect();

        hw.transaction(&mut hal_ops)?;

        Ok(buffers
            .into_iter()
            .map(TransactionBuffer::into_result)
            .collect())
    }

    fn drop(&mut self, rep: Resource<ActiveSpiDriver>) -> wasmtime::Result<()> {
        self.table.delete(rep)?;
        Ok(())
    }
}

pub fn add_to_linker<T: SpiView + 'static>(linker: &mut Linker<T>) -> wasmtime::Result<()> {
    spi::add_to_linker::<T, wasmtime::component::HasSelf<SpiCtx>>(linker, |host| host.spi_ctx())
}
