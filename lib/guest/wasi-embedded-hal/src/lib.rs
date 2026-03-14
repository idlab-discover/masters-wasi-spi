wit_bindgen::generate!({
    inline: "
        package guest:hal;
        world hal {
            import wasi:spi/spi;
            import wasi:gpio/gpio;
            import wasi:delay/delay;
        }
    ",
    path: "../../../guest/wit",
    generate_all
});

use embedded_hal::{
    delay::DelayNs,
    digital::{ErrorType as DigitalErrorType, OutputPin},
    spi::{Error as SpiErrorTrait, ErrorKind, ErrorType as SpiErrorType, Operation, SpiDevice},
};
use wasi::spi::spi::{Operation as WasiOp, OperationResult as WasiOpResult};

// ==========================================
// DELAY IMPLEMENTATION
// ==========================================

pub struct WasiDelay;

impl DelayNs for WasiDelay {
    fn delay_ns(&mut self, ns: u32) {
        let ms = (ns + 999_999) / 1_000_000;
        wasi::delay::delay::delay_ms(ms);
    }

    fn delay_us(&mut self, us: u32) {
        let ms = (us + 999) / 1_000;
        wasi::delay::delay::delay_ms(ms);
    }

    fn delay_ms(&mut self, ms: u32) {
        wasi::delay::delay::delay_ms(ms);
    }
}

// ==========================================
// GPIO IMPLEMENTATION
// ==========================================

pub struct WasiOutputPin {
    label: String,
}

impl WasiOutputPin {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
        }
    }
}

impl DigitalErrorType for WasiOutputPin {
    type Error = core::convert::Infallible;
}

impl OutputPin for WasiOutputPin {
    fn set_low(&mut self) -> Result<(), Self::Error> {
        wasi::gpio::gpio::set_pin_state(&self.label, wasi::gpio::gpio::Level::Low);
        Ok(())
    }

    fn set_high(&mut self) -> Result<(), Self::Error> {
        wasi::gpio::gpio::set_pin_state(&self.label, wasi::gpio::gpio::Level::High);
        Ok(())
    }
}

// ==========================================
// SPI IMPLEMENTATION
// ==========================================

pub struct WasiSpiDevice {
    inner: wasi::spi::spi::SpiDevice,
}

impl WasiSpiDevice {
    /// Create a wrapper from an already opened host SPI device
    pub fn new(inner: wasi::spi::spi::SpiDevice) -> Self {
        Self { inner }
    }

    /// Open an SPI device by name via the WASI host import
    pub fn open(name: &str) -> Result<Self, WasiSpiError> {
        let inner = wasi::spi::spi::open(name).map_err(WasiSpiError)?;
        Ok(Self { inner })
    }
}

#[derive(Debug)]
pub struct WasiSpiError(pub wasi::spi::spi::Error);

// Map the WASI host SPI errors to embedded-hal ErrorKind traits
impl SpiErrorTrait for WasiSpiError {
    fn kind(&self) -> ErrorKind {
        match &self.0 {
            wasi::spi::spi::Error::Overrun => ErrorKind::Overrun,
            wasi::spi::spi::Error::ModeFault => ErrorKind::ModeFault,
            wasi::spi::spi::Error::FrameFormat => ErrorKind::FrameFormat,
            wasi::spi::spi::Error::ChipSelectFault => ErrorKind::ChipSelectFault,
            wasi::spi::spi::Error::Other(_) => ErrorKind::Other,
        }
    }
}

impl core::fmt::Display for WasiSpiError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

impl SpiErrorType for WasiSpiDevice {
    type Error = WasiSpiError;
}

impl SpiDevice for WasiSpiDevice {
    fn transaction(&mut self, operations: &mut [Operation<'_, u8>]) -> Result<(), Self::Error> {
        let mut wasi_ops = Vec::with_capacity(operations.len());
        for op in operations.iter() {
            match op {
                Operation::Read(buf) => wasi_ops.push(WasiOp::Read(buf.len() as u64)),
                // Use .to_vec() since wit-bindgen requires owned Vec<u8> for these variants
                Operation::Write(buf) => wasi_ops.push(WasiOp::Write(buf.to_vec())),
                Operation::Transfer(_, write_buf) => {
                    wasi_ops.push(WasiOp::Transfer(write_buf.to_vec()))
                }
                Operation::TransferInPlace(words) => {
                    wasi_ops.push(WasiOp::Transfer(words.to_vec()))
                }
                Operation::DelayNs(ns) => wasi_ops.push(WasiOp::DelayNs(*ns)),
            }
        }

        let results = self.inner.transaction(&wasi_ops).map_err(WasiSpiError)?;

        let mut result_iter = results.into_iter();
        for op in operations.iter_mut() {
            match op {
                Operation::Read(buf) => {
                    if let Some(WasiOpResult::Read(data)) = result_iter.next() {
                        let len = data.len().min(buf.len());
                        buf[..len].copy_from_slice(&data[..len]);
                    }
                }
                Operation::Write(_) => {
                    result_iter.next();
                }
                Operation::Transfer(read_buf, _) => {
                    if let Some(WasiOpResult::Transfer(data)) = result_iter.next() {
                        let len = data.len().min(read_buf.len());
                        read_buf[..len].copy_from_slice(&data[..len]);
                    }
                }
                Operation::TransferInPlace(words) => {
                    if let Some(WasiOpResult::Transfer(data)) = result_iter.next() {
                        let len = data.len().min(words.len());
                        words[..len].copy_from_slice(&data[..len]);
                    }
                }
                Operation::DelayNs(_) => {
                    result_iter.next();
                }
            }
        }
        Ok(())
    }

    fn read(&mut self, words: &mut [u8]) -> Result<(), Self::Error> {
        let data = self.inner.read(words.len() as u64).map_err(WasiSpiError)?;
        let len = data.len().min(words.len());
        words[..len].copy_from_slice(&data[..len]);
        Ok(())
    }

    fn write(&mut self, words: &[u8]) -> Result<(), Self::Error> {
        self.inner.write(words).map_err(WasiSpiError)?;
        Ok(())
    }

    fn transfer(&mut self, read: &mut [u8], write: &[u8]) -> Result<(), Self::Error> {
        let data = self.inner.transfer(write).map_err(WasiSpiError)?;
        let len = data.len().min(read.len());
        read[..len].copy_from_slice(&data[..len]);
        Ok(())
    }

    fn transfer_in_place(&mut self, words: &mut [u8]) -> Result<(), Self::Error> {
        let data = self.inner.transfer(words).map_err(WasiSpiError)?;
        let len = data.len().min(words.len());
        words[..len].copy_from_slice(&data[..len]);
        Ok(())
    }
}
