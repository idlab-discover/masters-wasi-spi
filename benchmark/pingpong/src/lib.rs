#![no_std]
extern crate alloc;

use alloc::format;
use embedded_hal::spi::SpiDevice;

pub trait Timer {
    type Instant;
    fn now(&self) -> Self::Instant;
    fn elapsed_us(&self, start: Self::Instant) -> u64;
}

pub trait SpiConfigurator<SPI> {
    type Error;
    fn set_baud_rate(&mut self, spi: &mut SPI, baud: u32) -> Result<(), Self::Error>;
}

pub trait Logger {
    fn log(&mut self, msg: &str);
}

const MAX_BAUD: u32 = 32_000_000;
const MAX_SIZE: usize = 4096;
const FINE_STEP: u32 = 1_000;

pub fn run_benchmark_matrix<SPI, T, C, L>(
    spi: &mut SPI,
    timer: &T,
    configurator: &mut C,
    logger: &mut L,
    tx_buf: &[u8],
    rx_buf: &mut [u8],
    env_name: &str, // E.g. "Native" or "WASM"
) -> Result<(), SPI::Error>
where
    SPI: SpiDevice,
    T: Timer,
    C: SpiConfigurator<SPI>,
    L: Logger,
{
    // Limit the maximum size to 4096 or the capacity of the smallest buffer
    let limit_size = MAX_SIZE.min(tx_buf.len()).min(rx_buf.len());

    let mut size = 1;

    while size <= limit_size {
        let tx = &tx_buf[..size];
        
        let is_fine_grained = size == 1;
        let current_iterations = if is_fine_grained { 10_000 } else { 1000 };
        
        // Start fine-grained at 100k. Start coarse at 125k so doubling hits 250k exactly.
        let mut baud = if is_fine_grained { 100_000 } else { 125_000 };

        while baud <= MAX_BAUD {
            // Ask the environment to apply the baud rate
            let _ = configurator.set_baud_rate(spi, baud);

            let mut rx = &mut rx_buf[..size];
            rx.fill(0x00);
            
            let start = timer.now();

            for _ in 0..current_iterations {
                spi.transfer(&mut rx, tx)?;
            }

            let elapsed = timer.elapsed_us(start);
            let valid = rx == tx;
            let avg_us = elapsed as f64 / current_iterations as f64;

            logger.log(&format!(
                "{},{},{},{},{:.2},{}",
                env_name, baud, size, elapsed, avg_us, valid
            ));

            // Calculate the next baud rate
            if is_fine_grained {
                baud += FINE_STEP;
            } else {
                baud *= 2;
            }
        }

        size *= 2;
    }

    Ok(())
}
