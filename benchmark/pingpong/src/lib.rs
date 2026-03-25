use embedded_hal::spi::SpiDevice;

pub trait Timer {
    type Instant;
    fn now(&self) -> Self::Instant;
    fn elapsed_us(&self, start: Self::Instant) -> u64;
}

pub struct BenchmarkResult {
    pub packet_size: usize,
    pub iterations: usize,
    pub total_time_us: u64,
}

const SIZES: &[usize] = &[1, 4, 16, 64, 256, 1024];
const ITERATIONS: usize = 1000;

pub fn run_suite<SPI, T, F>(spi: &mut SPI, timer: &T, mut report: F) -> Result<(), SPI::Error>
where
    SPI: SpiDevice,
    T: Timer,
    F: FnMut(BenchmarkResult),
{
    let tx_buf = [0xA5; 1024]; // Dummy data to send
    let mut rx_buf = [0x00; 1024];

    for &size in SIZES {
        let tx = &tx_buf[..size];
        let mut rx = &mut rx_buf[..size];

        let start = timer.now();
        for _ in 0..ITERATIONS {
            spi.transfer(&mut rx, tx)?;
        }

        let elapsed = timer.elapsed_us(start);

        report(BenchmarkResult {
            packet_size: size,
            iterations: ITERATIONS,
            total_time_us: elapsed,
        });
    }

    Ok(())
}
