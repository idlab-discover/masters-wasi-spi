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
const BAUD_RATES: [u32; 56] = [
    // 10 kHz steps through the core anomaly zone (100 kHz - 500 kHz)
    100_000, 110_000, 120_000, 130_000, 140_000, 150_000, 160_000, 170_000, 180_000, 190_000,
    200_000, 210_000, 220_000, 230_000, 240_000, 250_000, 260_000, 270_000, 280_000, 290_000,
    300_000, 310_000, 320_000, 330_000, 340_000, 350_000, 360_000, 370_000, 380_000, 390_000,
    400_000, 410_000, 420_000, 430_000, 440_000, 450_000, 460_000, 470_000, 480_000, 490_000,
    500_000, // 50 kHz steps catching the tail end of the anomaly (550 kHz - 1 MHz)
    550_000, 600_000, 650_000, 700_000, 750_000, 800_000, 850_000, 900_000, 950_000, 1_000_000,
    // Massive jumps to find the exact polling flip (2 MHz - 32 MHz)
    2_000_000, 4_000_000, 8_000_000, 16_000_000, 32_000_000,
];

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
    let max_size = tx_buf.len().min(rx_buf.len());

    for &baud in &BAUD_RATES {
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
