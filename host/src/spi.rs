use embedded_hal::spi::SpiDevice;
use wasmtime::Result;
use wasmtime::component::Resource;

use crate::my_org::hardware::spi::{self, Error};

pub struct SpiImplementation<T>(pub T);

impl<T: SpiDevice + Send + Sync> spi::Host for SpiImplementation<T> {}

impl<T: SpiDevice + Send + Sync> spi::HostSpiDevice for SpiImplementation<T> {
    fn write(&mut self, _: Resource<spi::SpiDevice>, data: Vec<u8>) -> Result<(), Error> {
        // Use generic write
        match self.0.write(&data) {
            Ok(_) => Ok(()),
            Err(_) => Err(Error::Other),
        }
    }

    fn read(&mut self, _: Resource<spi::SpiDevice>, len: u64) -> Result<Vec<u8>, Error> {
        let mut buffer = vec![0u8; len as usize];
        match self.0.read(&mut buffer) {
            Ok(_) => Ok(buffer),
            Err(_) => Err(Error::Other),
        }
    }

    fn transfer(
        &mut self,
        _: Resource<spi::SpiDevice>,
        write: Vec<u8>,
        read_len: u64,
    ) -> Result<Vec<u8>, Error> {
        // embedded-hal requires read and write buffers to be the same length for transfer.
        // We calculate the max length to accommodate both.
        let len = std::cmp::max(write.len(), read_len as usize);

        // Prepare TX buffer (pad with zeros if read_len is larger)
        let mut tx_buf = write.clone();
        tx_buf.resize(len, 0);

        // Prepare RX buffer
        let mut rx_buf = vec![0u8; len];

        // Perform the transfer
        match self.0.transfer(&mut rx_buf, &tx_buf) {
            Ok(_) => {
                // Truncate result to the requested read length
                rx_buf.truncate(read_len as usize);
                Ok(rx_buf)
            }
            Err(_) => Err(Error::Other),
        }
    }

    fn transfer_in_place(
        &mut self,
        _: Resource<spi::SpiDevice>,
        data: Vec<u8>,
    ) -> Result<Vec<u8>, Error> {
        let mut buffer = data.clone();
        // transfer_in_place is a direct map
        match self.0.transfer_in_place(&mut buffer) {
            Ok(_) => Ok(buffer),
            Err(_) => Err(Error::Other),
        }
    }

    fn drop(&mut self, _rep: Resource<spi::SpiDevice>) -> Result<(), anyhow::Error> {
        Ok(())
    }
}
