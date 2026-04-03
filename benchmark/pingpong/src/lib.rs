// benchmark/pingpong/src/lib.rs
#![no_std]
extern crate alloc;

use alloc::format;
use embedded_hal::spi::SpiDevice;

pub trait Timer {
    type Instant;
    fn now(&self) -> Self::Instant;
    fn elapsed_us(&self, start: Self::Instant) -> u64;
}

// Injects the ability to configure baud rates for the specific SPI type
pub trait SpiConfigurator<SPI> {
    type Error;
    fn set_baud_rate(&mut self, spi: &mut SPI, baud: u32) -> Result<(), Self::Error>;
}

// Injects a logging capability
pub trait Logger {
    fn log(&mut self, msg: &str);
}

const ITERATIONS: usize = 100;
const START_BAUD: u32 = 50_000;
const END_BAUD: u32 = 1_000_000;
const STEP_BAUD: usize = 10_000;

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
    let max_size = 1;

    for baud in (START_BAUD..=END_BAUD).step_by(STEP_BAUD) {
        // Ask the environment to apply the baud rate
        let _ = configurator.set_baud_rate(spi, baud);

        let mut size = 1;
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
            let avg_us = elapsed as f64 / ITERATIONS as f64;

            logger.log(&format!(
                "{},{},{},{},{:.2},{}",
                env_name, baud, size, elapsed, avg_us, valid
            ));

            size *= 2;
        }
    }

    Ok(())
}
