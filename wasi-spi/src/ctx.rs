use linux_embedded_hal::SpidevDevice;
use std::collections::HashMap;
use std::marker::PhantomData;
use wasmtime::component::HasData;
use wasmtime_wasi::WasiView;

use crate::impls::SpiImpl;

#[derive(Clone)]
pub struct SpiConfig {
    pub virtual_name: String,
    pub physical_path: String,
}

pub struct WasiSpiCtx {
    pub devices: HashMap<String, String>,
}

impl WasiSpiCtx {
    pub fn from_configs(configs: Vec<SpiConfig>) -> anyhow::Result<Self> {
        let mut devices = HashMap::new();

        for config in configs {
            devices.insert(config.virtual_name, config.physical_path);
        }

        Ok(Self { devices })
    }
}

pub struct SpiDeviceState {
    pub device: SpidevDevice,
}

pub trait WasiSpiView: WasiView {
    fn spi_ctx(&mut self) -> &mut WasiSpiCtx;
}

pub struct Spi<T>(PhantomData<T>);

impl<T: WasiSpiView + 'static> HasData for Spi<T> {
    type Data<'a> = SpiImpl<'a, T>;
}
