use crate::mock_spi::MockSpiDevice;
use anyhow::Result;
use embedded_hal::spi::{
    ErrorKind as HalErrorKind, Operation as HalOperation, SpiDevice as HalSpiDevice,
};
use wasi::spi::spi as spi_bindings;
use wasmtime::component::{Resource, ResourceTable};
use wasmtime_wasi::{WasiCtx, WasiView};

wasmtime::component::bindgen!({
    path: "../example-guest-component/wit",
    world: "app",
});

pub use wasi::spi::spi::Error as WasiSpiError;

pub struct HostState {
    pub ctx: WasiCtx,
    pub table: ResourceTable,
}

impl WasiView for HostState {
    fn ctx(&mut self) -> wasmtime_wasi::WasiCtxView<'_> {
        wasmtime_wasi::WasiCtxView {
            ctx: &mut self.ctx,
            table: &mut self.table,
        }
    }
}

fn map_spi_error<E: embedded_hal::spi::Error>(err: E) -> WasiSpiError {
    match err.kind() {
        HalErrorKind::Overrun => WasiSpiError::Overrun,
        HalErrorKind::ModeFault => WasiSpiError::ModeFault,
        HalErrorKind::FrameFormat => WasiSpiError::FrameFormat,
        HalErrorKind::ChipSelectFault => WasiSpiError::ChipSelectFault,
        _ => WasiSpiError::Other,
    }
}

impl spi_bindings::Host for HostState {}

impl spi_bindings::HostSpiDevice for HostState {
    fn read(
        &mut self,
        res: Resource<spi_bindings::SpiDevice>,
        len: u64,
    ) -> Result<Vec<u8>, WasiSpiError> {
        let device = self
            .table
            .get_mut(&Resource::<MockSpiDevice>::new_borrow(res.rep()))
            .map_err(|_| WasiSpiError::Other)?;

        let mut buffer = vec![0u8; len as usize];
        device.read(&mut buffer).map_err(map_spi_error)?;
        Ok(buffer)
    }

    fn write(
        &mut self,
        res: Resource<spi_bindings::SpiDevice>,
        data: Vec<u8>,
    ) -> Result<(), WasiSpiError> {
        let device = self
            .table
            .get_mut(&Resource::<MockSpiDevice>::new_borrow(res.rep()))
            .map_err(|_| WasiSpiError::Other)?;
        device.write(&data).map_err(map_spi_error)
    }

    fn transfer(
        &mut self,
        res: Resource<spi_bindings::SpiDevice>,
        data: Vec<u8>,
    ) -> Result<Vec<u8>, WasiSpiError> {
        let device = self
            .table
            .get_mut(&Resource::<MockSpiDevice>::new_borrow(res.rep()))
            .map_err(|_| WasiSpiError::Other)?;

        let mut read_buffer = vec![0u8; data.len()];
        device
            .transfer(&mut read_buffer, &data)
            .map_err(map_spi_error)?;
        Ok(read_buffer)
    }

    fn transaction(
        &mut self,
        res: Resource<spi_bindings::SpiDevice>,
        operations: Vec<spi_bindings::Operation>,
    ) -> Result<Vec<Vec<u8>>, WasiSpiError> {
        let device = self
            .table
            .get_mut(&Resource::<MockSpiDevice>::new_borrow(res.rep()))
            .map_err(|_| WasiSpiError::Other)?;

        struct TransactionState {
            read_buf: Option<Vec<u8>>,
            write_buf: Option<Vec<u8>>,
            delay_ns: Option<u32>,
        }

        let mut states = Vec::with_capacity(operations.len());

        for op in operations {
            match op {
                spi_bindings::Operation::Read(len) => states.push(TransactionState {
                    read_buf: Some(vec![0u8; len as usize]),
                    write_buf: None,
                    delay_ns: None,
                }),
                spi_bindings::Operation::Write(data) => states.push(TransactionState {
                    read_buf: None,
                    write_buf: Some(data),
                    delay_ns: None,
                }),
                spi_bindings::Operation::Transfer(data) => states.push(TransactionState {
                    read_buf: Some(vec![0u8; data.len()]),
                    write_buf: Some(data),
                    delay_ns: None,
                }),
                spi_bindings::Operation::DelayNs(ns) => states.push(TransactionState {
                    read_buf: None,
                    write_buf: None,
                    delay_ns: Some(ns),
                }),
            }
        }

        let mut hal_ops: Vec<HalOperation<u8>> = states
            .iter_mut()
            .map(|state| {
                if let Some(ns) = state.delay_ns {
                    return HalOperation::DelayNs(ns);
                }
                match (&mut state.read_buf, &state.write_buf) {
                    (Some(r), Some(w)) => HalOperation::Transfer(r, w),
                    (Some(r), None) => HalOperation::Read(r),
                    (None, Some(w)) => HalOperation::Write(w),
                    _ => HalOperation::DelayNs(0),
                }
            })
            .collect();

        device.transaction(&mut hal_ops).map_err(map_spi_error)?;

        Ok(states
            .into_iter()
            .map(|s| s.read_buf.unwrap_or_default())
            .collect())
    }

    fn drop(&mut self, res: Resource<spi_bindings::SpiDevice>) -> Result<()> {
        self.table
            .delete::<MockSpiDevice>(Resource::new_own(res.rep()))?;
        Ok(())
    }
}
