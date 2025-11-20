use wasmtime::component::Resource;
use wasmtime::Result;
use std::io::{Read, Write};
use linux_embedded_hal::spidev::{Spidev, SpidevTransfer};

use crate::my_org::hardware::spi::{self, Error};

pub struct SpiImplementation(pub Spidev);

impl spi::Host for SpiImplementation {}

impl spi::HostSpiDevice for SpiImplementation {
    fn write(&mut self, _: Resource<spi::SpiDevice>, data: Vec<u8>) -> Result<(), Error> {
        match self.0.write_all(&data) {
            Ok(_) => Ok(()),
            Err(_) => Err(Error::Other),
        }
    }

    fn read(&mut self, _: Resource<spi::SpiDevice>, len: u64) -> Result<Vec<u8>, Error> {
        let mut buffer = vec![0u8; len as usize];
        // Use standard std::io::Read trait
        match self.0.read_exact(&mut buffer) {
            Ok(_) => Ok(buffer),
            Err(_) => Err(Error::Other),
        }
    }

    fn transfer(&mut self, _: Resource<spi::SpiDevice>, write: Vec<u8>, read_len: u64) -> Result<Vec<u8>, Error> {
        let mut read_buffer = vec![0u8; read_len as usize];

        // Create a low-level Linux SPI transfer
        let mut transfer = SpidevTransfer::read_write(&write, &mut read_buffer);

        // Execute using the inherent transfer method
        match self.0.transfer(&mut transfer) {
            Ok(_) => Ok(read_buffer),
            Err(_) => Err(Error::Other),
        }
    }

    fn transfer_in_place(&mut self, _: Resource<spi::SpiDevice>, data: Vec<u8>) -> Result<Vec<u8>, Error> {
        let tx_buf = data.clone();
        let mut rx_buf = data;

        let mut transfer = SpidevTransfer::read_write(&tx_buf, &mut rx_buf);

        match self.0.transfer(&mut transfer) {
            Ok(_) => Ok(rx_buf),
            Err(_) => Err(Error::Other),
        }
    }

    fn drop(&mut self, _rep: Resource<spi::SpiDevice>) -> Result<(), anyhow::Error> {
        Ok(())
    }
}