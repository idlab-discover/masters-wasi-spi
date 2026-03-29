#![no_std]

use embedded_hal::spi::SpiDevice;

pub trait Timer {
    type Instant;
    fn now(&self) -> Self::Instant;
    fn elapsed_us(&self, start: Self::Instant) -> u64;
}

#[derive(Default, Clone, Copy, Debug)] // Added Debug so you can easily print it
pub struct BenchmarkResult {
    pub packet_size: usize,
    pub iterations: usize,
    pub total_time_us: u64,
    pub valid_loopback: bool, // <--- New field to verify hardware loopback
}

pub const SIZES: [usize; 6] = [1, 4, 16, 64, 256, 1024];
const ITERATIONS: usize = 1000;

pub fn run_suite<SPI, T>(
    spi: &mut SPI,
    timer: &T,
) -> Result<[BenchmarkResult; SIZES.len()], SPI::Error>
where
    SPI: SpiDevice,
    T: Timer,
{
    // 0xA5 is binary 10100101, an excellent alternating bit pattern for testing SPI lines
    let tx_buf = [0xA5; 1024];
    let mut rx_buf = [0x00; 1024];

    let mut results = [BenchmarkResult::default(); SIZES.len()];

    for (i, &size) in SIZES.iter().enumerate() {
        let tx = &tx_buf[..size];
        let mut rx = &mut rx_buf[..size];

        // Wipe the RX buffer before starting to guarantee we aren't reading stale data
        rx.fill(0x00);

        // --- TIMING START ---
        let start = timer.now();

        for _ in 0..ITERATIONS {
            spi.transfer(&mut rx, tx)?;
        }

        let elapsed = timer.elapsed_us(start);
        // --- TIMING END ---

        // Verify that the data we just received perfectly matches what we transmitted
        let valid = rx == tx;

        results[i] = BenchmarkResult {
            packet_size: size,
            iterations: ITERATIONS,
            total_time_us: elapsed,
            valid_loopback: valid,
        };
    }

    Ok(results)
}
