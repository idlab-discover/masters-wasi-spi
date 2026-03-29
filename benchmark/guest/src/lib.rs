#![no_std]
extern crate alloc;

use pingpong::{Logger, SpiConfigurator, Timer, run_benchmark_matrix};
use wasi_embedded_hal::WasiSpiDevice;

wit_bindgen::generate!({
    path: "wit",
    world: "benchmark-app",
    generate_all,
});

struct WasiBenchEnv;

impl Timer for WasiBenchEnv {
    type Instant = u64;
    fn now(&self) -> Self::Instant {
        crate::my::timer::timer::now_micros()
    }
    fn elapsed_us(&self, start: Self::Instant) -> u64 {
        crate::my::timer::timer::now_micros() - start
    }
}

impl SpiConfigurator<WasiSpiDevice> for WasiBenchEnv {
    type Error = ();
    fn set_baud_rate(&mut self, _spi: &mut WasiSpiDevice, baud: u32) -> Result<(), Self::Error> {
        crate::wasi::benchmark::bench_utils::set_baud_rate(baud);
        Ok(())
    }
}

impl Logger for WasiBenchEnv {
    fn log(&mut self, msg: &str) {
        crate::wasi::benchmark::bench_utils::log(msg);
    }
}

struct BenchmarkGuest;

impl Guest for BenchmarkGuest {
    fn run_pingpong() {
        let mut spi = WasiSpiDevice::open("bench").expect("Failed to open SPI device");

        let timer_env = WasiBenchEnv;
        let mut config_env = WasiBenchEnv;
        let mut log_env = WasiBenchEnv;

        let tx_buf = alloc::vec![0xA5; 4096];
        let mut rx_buf = alloc::vec![0x00; 4096];

        run_benchmark_matrix(
            &mut spi,
            &timer_env,
            &mut config_env,
            &mut log_env,
            &tx_buf,
            &mut rx_buf,
            "WASM",
        )
        .expect("Failed WASM benchmark");
    }
}

export!(BenchmarkGuest);
