use embedded_hal::spi::{
    Error as HalError, ErrorKind as HalErrorKind, Operation as HalOperation,
    SpiDevice as HalSpiDevice,
};
use wasmtime::component::Resource;

pub mod bindings {
    use wasmtime::component::bindgen;
    bindgen!({
        path: "wit",
    });
}

use crate::bindings::wasi::spi::spi::{
    Error as WasiSpiError, Host, HostSpiDevice, Operation as WasiSpiOperation, SpiDevice,
};

pub struct SpiContext<T> {
    pub bus: T,
}

/// Helper function to map embedded-hal errors to WASM errors
fn map_spi_error<E: HalError>(err: E) -> WasiSpiError {
    match err.kind() {
        HalErrorKind::Overrun => WasiSpiError::Overrun,
        HalErrorKind::ModeFault => WasiSpiError::ModeFault,
        HalErrorKind::FrameFormat => WasiSpiError::FrameFormat,
        HalErrorKind::ChipSelectFault => WasiSpiError::ChipSelectFault,
        _ => WasiSpiError::Other,
    }
}

impl<T> Host for SpiContext<T> where T: HalSpiDevice + Send + Sync + 'static {}

impl<T> HostSpiDevice for SpiContext<T>
where
    T: HalSpiDevice + Send + Sync + 'static,
    T::Error: std::fmt::Debug,
{
    fn read(&mut self, _res: Resource<SpiDevice>, len: u64) -> Result<Vec<u8>, WasiSpiError> {
        let mut buffer = vec![0u8; len as usize];
        self.bus.read(&mut buffer).map_err(map_spi_error)?;
        Ok(buffer)
    }

    fn write(&mut self, _res: Resource<SpiDevice>, data: Vec<u8>) -> Result<(), WasiSpiError> {
        self.bus.write(&data).map_err(map_spi_error)
    }

    fn transfer(
        &mut self,
        _res: Resource<SpiDevice>,
        data: Vec<u8>,
    ) -> Result<Vec<u8>, WasiSpiError> {
        let mut read_buffer = vec![0u8; data.len()];
        self.bus
            .transfer(&mut read_buffer, &data)
            .map_err(map_spi_error)?;

        Ok(read_buffer)
    }

    fn transaction(
        &mut self,
        _res: Resource<SpiDevice>,
        operations: Vec<WasiSpiOperation>,
    ) -> Result<Vec<Vec<u8>>, WasiSpiError> {
        struct TransactionState {
            read_buf: Option<Vec<u8>>,
            write_buf: Option<Vec<u8>>,
            delay_ns: Option<u32>,
        }

        let mut states = Vec::with_capacity(operations.len());

        for op in operations {
            match op {
                WasiSpiOperation::Read(len) => {
                    states.push(TransactionState {
                        read_buf: Some(vec![0u8; len as usize]),
                        write_buf: None,
                        delay_ns: None,
                    });
                }
                WasiSpiOperation::Write(data) => {
                    states.push(TransactionState {
                        read_buf: None,
                        write_buf: Some(data),
                        delay_ns: None,
                    });
                }
                WasiSpiOperation::Transfer(data) => {
                    states.push(TransactionState {
                        read_buf: Some(vec![0u8; data.len()]),
                        write_buf: Some(data),
                        delay_ns: None,
                    });
                }
                WasiSpiOperation::DelayNs(ns) => {
                    states.push(TransactionState {
                        read_buf: None,
                        write_buf: None,
                        delay_ns: Some(ns),
                    });
                }
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

        self.bus.transaction(&mut hal_ops).map_err(map_spi_error)?;

        let results = states
            .into_iter()
            .map(|s| s.read_buf.unwrap_or_default())
            .collect();

        Ok(results)
    }

    fn drop(&mut self, _rep: Resource<SpiDevice>) -> anyhow::Result<()> {
        Ok(())
    }
}
