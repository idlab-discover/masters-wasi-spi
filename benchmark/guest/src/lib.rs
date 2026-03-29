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
    fn run_pingpong() -> Vec<(u32, u32, u64, bool)> {
        let mut spi = WasiSpiDevice::open("bench").expect("Failed to open SPI device");
        let timer = WasiTimer;

        // Define your max size here (e.g., 4096 bytes)
        let tx_buf = alloc::vec![0xA5; 4096];
        let mut rx_buf = alloc::vec![0x00; 4096];

        let mut results = Vec::new();

        // Pass the buffers and a closure to collect the results
        run_suite(&mut spi, &timer, &tx_buf, &mut rx_buf, |r| {
            results.push((
                r.packet_size as u32,
                r.iterations as u32,
                r.total_time_us,
                r.valid_loopback,
            ));
        })
        .expect("Failed to run benchmark suite");

        results
    }
}

export!(BenchmarkGuest);
