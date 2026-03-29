#![no_std]

use embedded_hal::spi::SpiDevice;

pub trait Timer {
    type Instant;
    fn now(&self) -> Self::Instant;
    fn elapsed_us(&self, start: Self::Instant) -> u64;
}

#[derive(Default, Clone, Copy, Debug)]
pub struct BenchmarkResult {
    pub packet_size: usize,
    pub iterations: usize,
    pub total_time_us: u64,
    pub valid_loopback: bool,
}

const ITERATIONS: usize = 1000;

// Now takes buffers dynamically and yields results via a callback closure
pub fn run_suite<SPI, T, F>(
    spi: &mut SPI,
    timer: &T,
    tx_buf: &[u8],
    rx_buf: &mut [u8],
    mut on_result: F,
) -> Result<(), SPI::Error>
where
    SPI: SpiDevice,
    T: Timer,
    F: FnMut(BenchmarkResult), // Callback for when a size test finishes
{
    // The max size is dictated purely by the slices passed in
    let max_size = tx_buf.len().min(rx_buf.len());
    let mut size = 1;

    // Start at 1, double every iteration until we hit max_size
    while size <= max_size {
        let tx = &tx_buf[..size];
        let mut rx = &mut rx_buf[..size];

        rx.fill(0x00);

        let start = timer.now();

        for _ in 0..ITERATIONS {
            spi.transfer(&mut rx, tx)?;
        }

        let elapsed = timer.elapsed_us(start);
        let valid = rx == tx;

        // Yield the result back to the caller
        on_result(BenchmarkResult {
            packet_size: size,
            iterations: ITERATIONS,
            total_time_us: elapsed,
            valid_loopback: valid,
        });

        size *= 2; // <--- Doubles the size for the next loop
    }

    Ok(())
}
