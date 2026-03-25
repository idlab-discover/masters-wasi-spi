use linux_embedded_hal::SpidevDevice;
use linux_embedded_hal::spidev::{SpiModeFlags, SpidevOptions};
use pingpong::{Timer, run_suite};
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

    let spi_path = "/dev/spidev0.0"; // Adjust to your setup
    let mut spi = SpidevDevice::open(spi_path).unwrap_or_else(|e| {
        panic!("Failed to open SPI device {}: {}", spi_path, e);
    });

    let timer = StdTimer;

    // The baud rates we want to test
    let baud_rates = [100_000, 500_000, 1_000_000, 5_000_000, 10_000_000];

    for baud in baud_rates {
        println!("\n--- Testing at {} Hz ---", baud);

        // Reconfigure the SPI bus for the new baud rate
        let options = SpidevOptions::new()
            .bits_per_word(8)
            .max_speed_hz(baud)
            .mode(SpiModeFlags::SPI_MODE_0)
            .build();

        spi.configure(&options)
            .expect("Failed to configure SPI options");

        // Run the suite
        match run_suite(&mut spi, &timer) {
            Ok(results) => {
                for res in results {
                    let avg_us = res.total_time_us as f64 / res.iterations as f64;
                    println!(
                        "Size: {:>4} bytes | Total: {:>8} µs | Avg RTT: {:>6.2} µs",
                        res.packet_size, res.total_time_us, avg_us
                    );
                }
            }
            Err(e) => eprintln!("Benchmark failed with SPI error: {:?}", e),
        }
    }
}
