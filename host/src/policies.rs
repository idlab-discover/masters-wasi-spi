use serde::Deserialize;

#[derive(Deserialize)]
pub struct HostPolicy {
    pub wasi: WasiPolicy,
}

#[derive(Deserialize)]
pub struct WasiPolicy {
    #[serde(default)]
    pub spi: Vec<SpiPolicyConfig>,
}

#[derive(Deserialize)]
pub struct SpiPolicyConfig {
    pub virtual_name: String,
    pub physical_path: String,
}
