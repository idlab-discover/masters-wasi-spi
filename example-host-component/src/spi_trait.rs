use embedded_hal::spi::{Operation, SpiDevice};

pub trait WasiSpiDevice: Send + Sync {
    fn read(&mut self, words: &mut [u8]) -> anyhow::Result<()>;
    fn write(&mut self, words: &[u8]) -> anyhow::Result<()>;
    fn transfer(&mut self, read: &mut [u8], write: &[u8]) -> anyhow::Result<()>;
    fn transaction(&mut self, operations: &mut [Operation<'_, u8>]) -> anyhow::Result<()>;
}

impl<T: SpiDevice + Send + Sync> WasiSpiDevice for T
where
    T::Error: std::error::Error + Send + Sync + 'static,
{
    fn read(&mut self, words: &mut [u8]) -> anyhow::Result<()> {
        SpiDevice::read(self, words).map_err(|e| anyhow::Error::new(e))
    }

    fn write(&mut self, words: &[u8]) -> anyhow::Result<()> {
        SpiDevice::write(self, words).map_err(|e| anyhow::Error::new(e))
    }

    fn transfer(&mut self, read: &mut [u8], write: &[u8]) -> anyhow::Result<()> {
        SpiDevice::transfer(self, read, write).map_err(|e| anyhow::Error::new(e))
    }

    fn transaction(&mut self, operations: &mut [Operation<'_, u8>]) -> anyhow::Result<()> {
        SpiDevice::transaction(self, operations).map_err(|e| anyhow::Error::new(e))
    }
}

pub struct SpiResource {
    pub device: Box<dyn WasiSpiDevice>,
}
