use anyhow::{Result, anyhow};
use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct HostArguments {
    /// The path to the WASM component to run
    #[arg(index = 1)]
    pub component_path: String,

    /// SPI Devices to pre-open.
    /// Format: "phys_path::virt_name::key=value..."
    /// Example: "/dev/spidev0.0::display::speed=8000000::mode=0"
    #[arg(long = "device", value_parser = parse_spi_config)]
    pub devices: Vec<SpiDeviceConfig>,
}

#[derive(Debug, Clone)]
pub struct SpiDeviceConfig {
    pub physical_path: String,
    pub virtual_name: String,
    pub max_speed_hz: u32,
    pub mode: u8, // 0-3
    pub lsb_first: bool,
    pub bits_per_word: u8,
}

impl Default for SpiDeviceConfig {
    fn default() -> Self {
        Self {
            physical_path: String::new(),
            virtual_name: String::new(),
            max_speed_hz: 1_000_000, // 1 MHz default
            mode: 0,
            lsb_first: false,
            bits_per_word: 8,
        }
    }
}

fn parse_spi_config(s: &str) -> Result<SpiDeviceConfig> {
    let parts: Vec<&str> = s.split("::").collect();
    if parts.len() < 2 {
        return Err(anyhow!("Invalid format. Expected 'path::name[::options]'"));
    }

    let mut config = SpiDeviceConfig::default();
    config.physical_path = parts[0].to_string();
    config.virtual_name = parts[1].to_string();

    for part in &parts[2..] {
        let kv: Vec<&str> = part.split('=').collect();
        if kv.len() != 2 {
            continue;
        }

        let key = kv[0];
        let value = kv[1];

        match key {
            "speed" => config.max_speed_hz = value.parse().map_err(|_| anyhow!("Invalid speed"))?,
            "mode" => config.mode = value.parse().map_err(|_| anyhow!("Invalid mode"))?,
            "lsb" => config.lsb_first = value.parse().map_err(|_| anyhow!("Invalid lsb bool"))?,
            "bits" => config.bits_per_word = value.parse().map_err(|_| anyhow!("Invalid bits"))?,
            _ => eprintln!("Warning: Unknown SPI config key '{}'", key),
        }
    }

    Ok(config)
}
