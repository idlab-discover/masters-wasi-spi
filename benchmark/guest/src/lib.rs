#![no_std]
extern crate alloc;

use alloc::vec::Vec;
use pingpong::{Timer, run_suite};
use wasi_embedded_hal::WasiSpiDevice;

wit_bindgen::generate!({
    path: "wit",
    world: "benchmark-app",
    generate_all,
});

struct WasiTimer;

impl Timer for WasiTimer {
    type Instant = u64;

    fn now(&self) -> Self::Instant {
        crate::my::timer::timer::now_micros()
    }

    fn elapsed_us(&self, start: Self::Instant) -> u64 {
        crate::my::timer::timer::now_micros() - start
    }
}

struct BenchmarkGuest;

impl Guest for BenchmarkGuest {
    fn run_pingpong() -> Vec<(u32, u32, u64)> {
        // Open SPI device via the WASI import
        let mut spi = WasiSpiDevice::open("bench").expect("Failed to open SPI device");
        let timer = WasiTimer;

        // Run the pingpong suite
        let results = run_suite(&mut spi, &timer).expect("Failed to run benchmark suite");

        // Map the results into the flat tuple structure expected by WIT
        results
            .iter()
            .map(|r| (r.packet_size as u32, r.iterations as u32, r.total_time_us))
            .collect()
    }
}

export!(BenchmarkGuest);
