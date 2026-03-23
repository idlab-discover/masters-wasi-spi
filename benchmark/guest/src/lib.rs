#![no_std]
extern crate alloc;

use crate::my::timer::timer;
use alloc::vec;

wit_bindgen::generate!({
    path: "wit",
    world: "benchmark-app",
    generate_all,
});

struct BenchmarkGuest;

impl Guest for BenchmarkGuest {
    fn run_benchmark(payload_size: u32, iterations: u32) -> u64 {
        let spi = wasi::spi::spi::open("bench").expect("Failed to open SPI device");
        let payload = vec![0xAA; payload_size as usize];
        let start_time = timer::now_micros();

        for _ in 0..iterations {
            let _ = spi.write(&payload);
        }

        let end_time = timer::now_micros();

        end_time - start_time
    }
}

export!(BenchmarkGuest);
