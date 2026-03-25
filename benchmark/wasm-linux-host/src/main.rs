use linux_embedded_hal::SpidevDevice;
use linux_embedded_hal::spidev::{SpiModeFlags, SpidevOptions};
use spi::{ErasedSpiDevice, SpiCtx, SpiView};
use std::time::Instant;
use wasmtime::{
    Config, Engine, Store,
    component::{Component, HasSelf, Linker, ResourceTable},
};

wasmtime::component::bindgen!({
    path: "../guest/wit",
    world: "benchmark-app",
});

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

    let component = Component::from_file(
        &engine,
        "target/wasm32-unknown-unknown/release/benchmark_guest.component.wasm",
    )?;

    println!("=== Starting WASI Linux SPI Benchmark ===");

    let spi_path = "/dev/spidev0.0"; // Adjust to your setup
    let baud_rates = [100_000, 500_000, 1_000_000, 5_000_000, 10_000_000];

    for baud in baud_rates {
        println!("\n--- Testing at {} Hz ---", baud);

        // 1. Open and reconfigure the physical SPI bus for the new baud rate
        let mut spi = SpidevDevice::open(spi_path).unwrap_or_else(|e| {
            panic!("Failed to open SPI device {}: {}", spi_path, e);
        });

        let options = SpidevOptions::new()
            .bits_per_word(8)
            .max_speed_hz(baud)
            .mode(SpiModeFlags::SPI_MODE_0)
            .build();

        spi.configure(&options)
            .expect("Failed to configure SPI options");

        // 2. Erase the type so it can be passed into the WASI context
        let mut spi_hardware: Vec<(String, Box<dyn ErasedSpiDevice + Send + 'static>)> = vec![];
        spi_hardware.push((
            "bench".to_string(),
            Box::new(spi) as Box<dyn ErasedSpiDevice + Send + 'static>,
        ));

        let state = HostState {
            spi_ctx: SpiCtx {
                table: ResourceTable::new(),
                hardware: spi_hardware,
            },
            app_start_time: Instant::now(),
        };

        // 3. Setup store and Linker
        let mut store = Store::new(&engine, state);
        let mut linker = Linker::new(&engine);

        spi::add_to_linker(&mut linker)?;
        crate::my::timer::timer::add_to_linker::<HostState, HasSelf<HostState>>(
            &mut linker,
            |state| state,
        )?;

        // 4. Instantiate and call the Guest payload
        let app = BenchmarkApp::instantiate(&mut store, &component, &linker)?;
        let results = app.call_run_pingpong(&mut store)?;

        // 5. Output benchmark results
        for (packet_size, iterations, total_time_us) in results {
            let avg_us = total_time_us as f64 / iterations as f64;
            println!(
                "Size: {:>4} bytes | Total: {:>8} µs | Avg RTT: {:>6.2} µs",
                packet_size, total_time_us, avg_us
            );
        }
    }

    Ok(())
}
