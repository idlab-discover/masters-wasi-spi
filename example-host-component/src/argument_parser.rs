use anyhow::{Result, anyhow};
use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about)]
pub struct HostArguments {
    /// Path to the WASM component
    #[arg(index = 1)]
    pub component_path: String,

    /// Pre-open SPI devices: "phys_path::virt_name"
    #[arg(long = "device", value_parser = parse_spi_device)]
    pub devices: Vec<SpiDeviceMapping>,

    /// Path to the GPIO policy TOML file
    #[arg(long = "policy-file")]
    pub policy_file: String,
}

#[derive(Debug, Clone)]
pub struct SpiDeviceMapping {
    pub physical_path: String,
    pub virtual_name: String,
}

fn parse_spi_device(s: &str) -> Result<SpiDeviceMapping> {
    let (physical_path, virtual_name) = s
        .split_once("::")
        .ok_or_else(|| anyhow!("Invalid device format. Expected 'phys::virt'"))?;

    if physical_path.is_empty() || virtual_name.is_empty() {
        return Err(anyhow!("Device path and virtual name must be non-empty"));
    }

    Ok(SpiDeviceMapping {
        physical_path: physical_path.to_string(),
        virtual_name: virtual_name.to_string(),
    })
}
