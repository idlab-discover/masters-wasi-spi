use embedded_hal::spi::{self, Operation, SpiDevice, ErrorType};

#[derive(Debug)]
pub struct MockSpi;

#[derive(Debug)]
pub struct MockError;
impl spi::Error for MockError {
    fn kind(&self) -> spi::ErrorKind { spi::ErrorKind::Other }
}
impl std::fmt::Display for MockError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "Mock Error") }
}
impl std::error::Error for MockError {}

impl ErrorType for MockSpi {
    type Error = MockError;
}

// 3. The Implementation
impl SpiDevice<u8> for MockSpi {
    fn transaction(&mut self, operations: &mut [Operation<'_, u8>]) -> Result<(), Self::Error> {
        // We MUST loop through operations because 'read' and 'write' calls
        // are passed to us as a list of operations.
        for op in operations {
            match op {
                // "Read reads the amount of hardcoded bytes"
                Operation::Read(buf) => {
                    // Fill the buffer with a constant value (e.g., 0x42)
                    buf.fill(0x42);
                    println!("Mock SPI: Read {} bytes (returning 0x42)", buf.len());
                },
                // "Write always returns ok"
                Operation::Write(buf) => {
                    println!("Mock SPI: Wrote {:2X?}", buf);
                },
                // Handle transfers (simultaneous read/write) if needed
                Operation::Transfer(read, write) => {
                    println!("Mock SPI: Transfer (Write {:?}, Read {})", write, read.len());
                    read.fill(0x42);
                },
                // Ignore other fancy operations for simplicity
                Operation::TransferInPlace(buf) => {
                    println!("Mock SPI: TransferInPlace (Buf len {})", buf.len());
                    buf.fill(0x42);
                },
                _ => {}
            }
        }
        Ok(())
    }
}