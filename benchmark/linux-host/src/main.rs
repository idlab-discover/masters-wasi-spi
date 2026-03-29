// benchmark/linux-host/src/main.rs (The single collapsed host)
use linux_embedded_hal::SpidevDevice;
use linux_embedded_hal::spidev::{SpiModeFlags, SpidevOptions};
use pingpong::{Logger, SpiConfigurator, Timer, run_benchmark_matrix};
use spi::{ErasedSpiDevice, SpiCtx, SpiView};
use std::time::Instant;
use wasmtime::{
    Config, Engine, Store,
    component::{Component, HasSelf, Linker, ResourceTable},
};

// ----- Native Benchmark Implementations -----
struct NativeBenchEnv;

impl Timer for NativeBenchEnv {
    type Instant = Instant;
    fn now(&self) -> Self::Instant {
        Instant::now()
    }
    fn elapsed_us(&self, start: Self::Instant) -> u64 {
        start.elapsed().as_micros() as u64
    }
}

impl SpiConfigurator<SpidevDevice> for NativeBenchEnv {
    type Error = std::io::Error;
    fn set_baud_rate(&mut self, spi: &mut SpidevDevice, baud: u32) -> Result<(), Self::Error> {
        let options = SpidevOptions::new()
            .bits_per_word(8)
            .max_speed_hz(baud)
            .mode(SpiModeFlags::SPI_MODE_0)
            .build();
        spi.configure(&options)
    }
}

impl Logger for NativeBenchEnv {
    fn log(&mut self, msg: &str) {
        println!("{}", msg);
    }
}

// ----- WASM Host State -----
wasmtime::component::bindgen!({
    path: "../guest/wit",
    world: "benchmark-app",
});

struct HostState {
    spi_ctx: SpiCtx,
    app_start_time: Instant,
    spi_path: String,
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

impl crate::wasi::benchmark::bench_utils::Host for HostState {
    fn set_baud_rate(&mut self, baud: u32) {
        // Open a secondary handle just to configure the device ioctls
        if let Ok(mut spi) = SpidevDevice::open(&self.spi_path) {
            let options = SpidevOptions::new()
                .bits_per_word(8)
                .max_speed_hz(baud)
                .mode(SpiModeFlags::SPI_MODE_0)
                .build();
            let _ = spi.configure(&options);
        }
    }

    fn log(&mut self, msg: String) {
        println!("{}", msg);
    }
}

fn main() -> anyhow::Result<()> {
    let spi_path = "/dev/spidev0.0";
    let tx_buf = vec![0xA5; 4096];
    let mut rx_buf = vec![0x00; 4096];

    println!("Environment,BaudRate,Size_Bytes,TotalTime_us,AvgRTT_us,LoopbackValid");

    // =====================================
    // 1. Run Native Context
    // =====================================
    {
        let mut spi = SpidevDevice::open(spi_path).expect("Failed to open SPI for native");

        // Instantiate a separate ZST for each trait
        let timer_env = NativeBenchEnv;
        let mut config_env = NativeBenchEnv;
        let mut log_env = NativeBenchEnv;

        run_benchmark_matrix(
            &mut spi,
            &timer_env,
            &mut config_env,
            &mut log_env,
            &tx_buf,
            &mut rx_buf,
            "Native",
        )
        .expect("Native benchmark failed");
    }

    println!("\n");

    // =====================================
    // 2. Run Wasm Context
    // =====================================
    {
        let engine = Engine::new(&Config::new())?;
        let component = Component::from_file(
            &engine,
            "target/wasm32-unknown-unknown/release/benchmark_guest.component.wasm",
        )?;

        let spi = SpidevDevice::open(spi_path).unwrap();
        let mut spi_hardware: Vec<(String, Box<dyn ErasedSpiDevice + Send + 'static>)> = vec![];
        spi_hardware.push(("bench".to_string(), Box::new(spi)));

        let state = HostState {
            spi_ctx: SpiCtx {
                table: ResourceTable::new(),
                hardware: spi_hardware,
            },
            app_start_time: Instant::now(),
            spi_path: spi_path.to_string(),
        };

        let mut store = Store::new(&engine, state);
        let mut linker = Linker::new(&engine);

        spi::add_to_linker(&mut linker)?;

        // Use HasSelf to satisfy the trait bounds for type inference
        crate::my::timer::timer::add_to_linker::<HostState, HasSelf<HostState>>(
            &mut linker,
            |state| state,
        )?;
        crate::wasi::benchmark::bench_utils::add_to_linker::<HostState, HasSelf<HostState>>(
            &mut linker,
            |state| state,
        )?;

        let app = BenchmarkApp::instantiate(&mut store, &component, &linker)?;

        // Starts the run matrix directly inside the guest
        app.call_run_pingpong(&mut store)?;
    }

    Ok(())
}
