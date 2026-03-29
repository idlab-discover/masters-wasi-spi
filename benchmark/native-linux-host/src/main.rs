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
    let spi_path = "/dev/spidev0.0";
    let mut spi = SpidevDevice::open(spi_path).expect("Failed to open SPI device");
    let timer = StdTimer;

    // Finer, wider baud rates
    let baud_rates = [
        100_000, 500_000, 1_000_000, 2_000_000, 5_000_000, 10_000_000, 15_000_000, 20_000_000,
    ];

    // The host buffers (max size)
    let tx_buf = vec![0xA5; 4096];
    let mut rx_buf = vec![0x00; 4096];

    // Print CSV Header
    println!("BaudRate,Size_Bytes,TotalTime_us,AvgRTT_us,LoopbackValid");

    for baud in baud_rates {
        let options = SpidevOptions::new()
            .bits_per_word(8)
            .max_speed_hz(baud)
            .mode(SpiModeFlags::SPI_MODE_0)
            .build();

        spi.configure(&options).expect("Failed to configure SPI");

        // Use the closure to print CSV rows as they finish
        if let Err(e) = run_suite(&mut spi, &timer, &tx_buf, &mut rx_buf, |res| {
            let avg_us = res.total_time_us as f64 / res.iterations as f64;
            println!(
                "{},{},{},{:.2},{}",
                baud, res.packet_size, res.total_time_us, avg_us, res.valid_loopback
            );
        }) {
            eprintln!("Benchmark failed with SPI error: {:?}", e);
        }
    }
}
