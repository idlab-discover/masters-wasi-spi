use anyhow::Result;
use embedded_hal::spi::{
    ErrorKind as HalErrorKind, Operation as HalOperation, SpiDevice as HalSpiDevice,
};
use wasmtime::component::{Component, HasSelf, Linker, Resource, ResourceTable};
use wasmtime::{Config, Engine, Store};
use wasmtime_wasi::p2::add_to_linker_sync;
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView};

// 1. Generate bindings.
// This generates the 'wasi' module containing the traits we need to implement.
wasmtime::component::bindgen!({
    path: "../example-guest-component/wit",
    world: "app",
});

mod mock_spi;
use mock_spi::MockSpiDevice;

// Alias for the generated WASI SPI Error type
use wasi::spi::spi::Error as WasiSpiError;

// 2. Define the HostState
pub struct HostState {
    ctx: WasiCtx,
    table: ResourceTable,
}

// 3. Implement WasiView (standard requirement)
impl WasiView for HostState {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.ctx,
            table: &mut self.table,
        }
    }
}

// 4. Helper to map HAL errors to WASI errors
fn map_spi_error<E: embedded_hal::spi::Error>(err: E) -> WasiSpiError {
    match err.kind() {
        HalErrorKind::Overrun => WasiSpiError::Overrun,
        HalErrorKind::ModeFault => WasiSpiError::ModeFault,
        HalErrorKind::FrameFormat => WasiSpiError::FrameFormat,
        HalErrorKind::ChipSelectFault => WasiSpiError::ChipSelectFault,
        _ => WasiSpiError::Other,
    }
}

// 5. Implement the generated Host trait (marker trait)
impl wasi::spi::spi::Host for HostState {}

// 6. Implement the main HostSpiDevice trait directly on HostState
impl wasi::spi::spi::HostSpiDevice for HostState {
    fn read(
        &mut self,
        res: Resource<wasi::spi::spi::SpiDevice>,
        len: u64,
    ) -> Result<Vec<u8>, WasiSpiError> {
        // Use get_mut to get a mutable reference needed for SPI operations
        // Cast the generic WIT resource handle to the specific host resource handle
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
        device
            .transfer(&mut read_buffer, &data)
            .map_err(map_spi_error)?;
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

        // Map internal state to embedded-hal operations
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

        // Execute transaction
        device.transaction(&mut hal_ops).map_err(map_spi_error)?;

        // Collect results
        Ok(states
            .into_iter()
            .map(|s| s.read_buf.unwrap_or_default())
            .collect())
    }

    fn drop(&mut self, res: Resource<wasi::spi::spi::SpiDevice>) -> anyhow::Result<()> {
        // Cast resource for deletion
        self.table
            .delete::<MockSpiDevice>(Resource::new_own(res.rep()))?;
        Ok(())
    }
}

pub fn main() -> Result<()> {
    let guest_path = "../target/wasm32-wasip2/release/example_guest_component.wasm";

    let mut config = Config::new();
    config.wasm_component_model(true);
    let engine = Engine::new(&config)?;
    let mut linker = Linker::new(&engine);

    add_to_linker_sync(&mut linker)?;

    // Link the SPI interface to our HostState implementation
    // Added explicit Generic types <HostState, HostState> to help type inference
    wasi::spi::spi::add_to_linker::<HostState, HasSelf<HostState>>(&mut linker, |state: &mut HostState| state)?;

    let mut state = HostState {
        ctx: WasiCtxBuilder::new().inherit_stdio().build(),
        table: ResourceTable::new(),
    };

    // --- Add Multiple Devices ---
    let device1 = MockSpiDevice;
    // let device2 = MockSpiDevice; // You can add more devices here

    let dev1_resource = state.table.push(device1)?;

    let device_handle = Resource::<wasi::spi::spi::SpiDevice>::new_own(dev1_resource.rep());

    let mut store = Store::new(&engine, state);
    let component = Component::from_file(&engine, guest_path)?;
    let instance = App::instantiate(&mut store, &component, &linker)?;

    println!("Host: Calling guest run()...");
    instance.call_run(&mut store, device_handle)?;
    println!("Host: Guest finished.");

    Ok(())
}