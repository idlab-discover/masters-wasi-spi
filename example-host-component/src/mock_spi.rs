use embedded_hal::spi::{Error, ErrorKind, ErrorType, Operation, SpiDevice};

#[derive(Clone)]
pub struct MockSpiDevice;

#[derive(Debug)]
pub enum MockSpiError {}

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
        // You can iterate through operations here if you want to inspect data
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
                    read.copy_from_slice(&write[..read.len()]); // Loopback example
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
