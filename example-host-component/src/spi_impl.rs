// src/spi_impl.rs
use anyhow::Result;
use wasmtime::component::{Resource, ResourceTable};
use wasmtime_wasi::{WasiCtx, WasiView};
use embedded_hal::spi::{ErrorKind as HalErrorKind, Operation as HalOperation, SpiDevice as HalSpiDevice};

// Import MockSpiDevice from the crate root
use crate::mock_spi::MockSpiDevice;

// 1. Generate bindings here.
// The 'App' struct and 'wasi' module will be generated inside this module.
wasmtime::component::bindgen!({
    path: "../example-guest-component/wit",
    world: "app",
});

// Alias for the generated WASI SPI Error type
pub use wasi::spi::spi::Error as WasiSpiError;

// 2. Define the HostState
// Make fields public or provide a constructor so main.rs can initialize it.
pub struct HostState {
    pub ctx: WasiCtx,
    pub table: ResourceTable,
}

// 3. Implement WasiView
impl WasiView for HostState {
    fn ctx(&mut self) -> wasmtime_wasi::WasiCtxView<'_> {
        wasmtime_wasi::WasiCtxView {
            ctx: &mut self.ctx,
            table: &mut self.table,
        }
    }
}

// 4. Helper to map HAL errors
fn map_spi_error<E: embedded_hal::spi::Error>(err: E) -> WasiSpiError {
    match err.kind() {
        HalErrorKind::Overrun => WasiSpiError::Overrun,
        HalErrorKind::ModeFault => WasiSpiError::ModeFault,
        HalErrorKind::FrameFormat => WasiSpiError::FrameFormat,
        HalErrorKind::ChipSelectFault => WasiSpiError::ChipSelectFault,
        _ => WasiSpiError::Other,
    }
}

// 5. Implement the generated Host trait
impl wasi::spi::spi::Host for HostState {}

// 6. Implement HostSpiDevice
impl wasi::spi::spi::HostSpiDevice for HostState {
    fn read(
        &mut self,
        res: Resource<wasi::spi::spi::SpiDevice>,
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
        res: Resource<wasi::spi::spi::SpiDevice>,
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
        res: Resource<wasi::spi::spi::SpiDevice>,
        data: Vec<u8>,
    ) -> Result<Vec<u8>, WasiSpiError> {
        let device = self
            .table
            .get_mut(&Resource::<MockSpiDevice>::new_borrow(res.rep()))
            .map_err(|_| WasiSpiError::Other)?;

        let mut read_buffer = vec![0u8; data.len()];
        device.transfer(&mut read_buffer, &data).map_err(map_spi_error)?;
        Ok(read_buffer)
    }

    fn transaction(
        &mut self,
        res: Resource<wasi::spi::spi::SpiDevice>,
        operations: Vec<wasi::spi::spi::Operation>,
    ) -> Result<Vec<Vec<u8>>, WasiSpiError> {
        let device = self
            .table
            .get_mut(&Resource::<MockSpiDevice>::new_borrow(res.rep()))
            .map_err(|_| WasiSpiError::Other)?;

        // Temporary storage for transaction buffers
        struct TransactionState {
            read_buf: Option<Vec<u8>>,
            write_buf: Option<Vec<u8>>,
            delay_ns: Option<u32>,
        }

        let mut states = Vec::with_capacity(operations.len());

        for op in operations {
            match op {
                wasi::spi::spi::Operation::Read(len) => states.push(TransactionState {
                    read_buf: Some(vec![0u8; len as usize]),
                    write_buf: None,
                    delay_ns: None,
                }),
                wasi::spi::spi::Operation::Write(data) => states.push(TransactionState {
                    read_buf: None,
                    write_buf: Some(data),
                    delay_ns: None,
                }),
                wasi::spi::spi::Operation::Transfer(data) => states.push(TransactionState {
                    read_buf: Some(vec![0u8; data.len()]),
                    write_buf: Some(data),
                    delay_ns: None,
                }),
                wasi::spi::spi::Operation::DelayNs(ns) => states.push(TransactionState {
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

    fn drop(&mut self, res: Resource<wasi::spi::spi::SpiDevice>) -> anyhow::Result<()> {
        self.table.delete::<MockSpiDevice>(Resource::new_own(res.rep()))?;
        Ok(())
    }
}