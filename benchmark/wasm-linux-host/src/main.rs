use spi::{ErasedSpiDevice, SpiCtx, SpiView};
use std::time::Instant;
use wasmtime::{
    Config, Engine, Store,
    component::{Component, HasSelf, Linker, ResourceTable},
};

// We use embedded-hal traits to create a mock device
use embedded_hal::spi::{Error as HalError, ErrorKind, ErrorType, Operation, SpiDevice};

wasmtime::component::bindgen!({
    path: "../guest/wit",
    world: "benchmark-app",
});

// --- Mock SPI Implementation ---
pub struct MockSpi;

#[derive(Debug)]
pub struct MockError;

impl HalError for MockError {
    fn kind(&self) -> ErrorKind {
        ErrorKind::Other
    }
}

impl ErrorType for MockSpi {
    type Error = MockError;
}

impl SpiDevice for MockSpi {
    fn transaction(&mut self, _operations: &mut [Operation<'_, u8>]) -> Result<(), Self::Error> {
        Ok(())
    }
    fn read(&mut self, _words: &mut [u8]) -> Result<(), Self::Error> {
        Ok(())
    }
    fn write(&mut self, _words: &[u8]) -> Result<(), Self::Error> {
        Ok(())
    }
    fn transfer(&mut self, _read: &mut [u8], _write: &[u8]) -> Result<(), Self::Error> {
        Ok(())
    }
}
// -------------------------------

struct HostState {
    spi_ctx: SpiCtx,
    app_start_time: Instant,
}

impl SpiView for HostState {
    fn spi_ctx(&mut self) -> &mut SpiCtx {
        &mut self.spi_ctx
    }
}

impl crate::my::timer::timer::Host for HostState {
    fn now_micros(&mut self) -> u64 {
        self.app_start_time.elapsed().as_micros() as u64
    }
}

fn main() -> anyhow::Result<()> {
    let engine = Engine::new(&Config::new())?;

    // Note: Kept your exact component path
    let component = Component::from_file(
        &engine,
        "target/wasm32-unknown-unknown/release/benchmark_guest.component.wasm",
    )?;

    // We keep the baud rates loop just to generate the same formatted output,
    // but the actual hardware is mocked so it will hit the maximum possible throughput every time.
    let payload_sizes: [u32; 3] = [16, 128, 1024];
    let iterations: u32 = 10_000;

    println!("Starting Linux Cranelift SPI Benchmark\n======================================");

    for size in payload_sizes {
        // Replaced real spidev with the MockSpi!
        let mut spi_hardware: Vec<(String, Box<dyn ErasedSpiDevice + Send + 'static>)> = vec![];
        spi_hardware.push((
            "bench".to_string(),
            Box::new(MockSpi) as Box<dyn ErasedSpiDevice + Send + 'static>,
        ));

        let state = HostState {
            spi_ctx: SpiCtx {
                table: ResourceTable::new(),
                hardware: spi_hardware,
            },
            app_start_time: Instant::now(),
        };

        let mut store = Store::new(&engine, state);
        let mut linker = Linker::new(&engine);

        spi::add_to_linker(&mut linker)?;
        crate::my::timer::timer::add_to_linker::<HostState, HasSelf<HostState>>(
            &mut linker,
            |state| state,
        )?;

        let app = BenchmarkApp::instantiate(&mut store, &component, &linker)?;
        let time_micros = app.call_run_benchmark(&mut store, size, iterations)?;

        let total_bits = (size as f64 * iterations as f64) * 8.0;
        let throughput_mbps = total_bits / time_micros as f64;

        println!(
            "Payload: {:>4} B | Iterations: {:>5} | Time: {:>7} µs | Throughput: {:>5.2} Mbps",
            size, iterations, time_micros, throughput_mbps
        );
    }
    Ok(())
}
