use core::fmt;
use embedded_hal::spi::{Error, ErrorKind, ErrorType, Operation, SpiDevice};

#[derive(Clone)]
pub struct MockSpiDevice;

#[derive(Debug)]
pub enum MockSpiError {}

impl fmt::Display for MockSpiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Simulated Mock SPI Error")
    }
}

impl std::error::Error for MockSpiError {}

impl Error for MockSpiError {
    fn kind(&self) -> ErrorKind {
        ErrorKind::Other
    }
}

impl ErrorType for MockSpiDevice {
    type Error = MockSpiError;
}

impl SpiDevice for MockSpiDevice {
    fn transaction(&mut self, operations: &mut [Operation<'_, u8>]) -> Result<(), Self::Error> {
        for op in operations {
            match op {
                Operation::Read(buf) => {
                    println!("MockSpiImpl: Transaction Read: {} bytes", buf.len());
                    buf.fill(0xAA);
                }
                Operation::Write(buf) => {
                    println!("MockSpiImpl: Transaction Write: {:?}", buf);
                }
                Operation::Transfer(read, write) => {
                    println!("MockSpiImpl: Transaction Transfer");

                    let len = read.len().min(write.len());
                    read[..len].copy_from_slice(&write[..len]);
                }
                Operation::TransferInPlace(_buf) => {
                    println!("MockSpiImpl: Transaction Transfer In Place");
                }
                Operation::DelayNs(ns) => {
                    println!("MockSpiImpl: Delaying for {} ns", ns);
                }
            }
        }

        Ok(())
    }
}
