use linux_embedded_hal::SpidevDevice;
use linux_embedded_hal::spidev::{SpiModeFlags, SpidevOptions};
use pingpong::{BenchmarkResult, Timer, run_suite};
use std::time::Instant;

struct StdTimer;

impl Timer for StdTimer {
    type Instant = Instant;

    fn now(&self) -> Self::Instant {
        Instant::now()
    }

    fn elapsed_us(&self, start: Self::Instant) -> u64 {
        start.elapsed().as_micros() as u64
    }
}

fn main() {
    println!("=== Starting Native Linux SPI Benchmark ===");

    let spi_path = "/dev/spidev0.0";

    let mut spi = SpidevDevice::open(spi_path).unwrap_or_else(|e| {
        panic!("Failed to open SPI device {}: {}", spi_path, e);
    });

    let options = SpidevOptions::new()
        .bits_per_word(8)
        .max_speed_hz(5_000_000) // 5 MHz - adjust as needed
        .mode(SpiModeFlags::SPI_MODE_0)
        .build();

    spi.configure(&options)
        .expect("Failed to configure SPI options");

    let timer = StdTimer;

    let result = run_suite(&mut spi, &timer, |res: BenchmarkResult| {
        let avg_us = res.total_time_us as f64 / res.iterations as f64;

        println!(
            "Size: {:>4} bytes | Total time: {:>8} µs | Avg per transfer: {:>6.2} µs",
            res.packet_size, res.total_time_us, avg_us
        );
    });

    match result {
        Ok(_) => println!("Benchmark completed successfully!"),
        Err(e) => eprintln!("Benchmark failed with SPI error: {:?}", e),
    }
}
