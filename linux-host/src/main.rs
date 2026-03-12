use std::collections::BTreeMap;
use std::fs;

use anyhow::Context; // For helpful error messages
use clap::Parser;
use serde::Deserialize;
use wasmtime::{
    Config, Engine, Store,
    component::{Component, HasSelf, Linker, ResourceTable},
};

// Import your shared, hardware-agnostic libraries
use delay::{DelayCtx, DelayView};
use gpio::{GpioCtx, GpioView};
use spi::{SpiCtx, SpiView};

// Linux hardware implementations for embedded-hal (Modern CDEV approach)
use linux_embedded_hal::gpio_cdev::{Chip, LineRequestFlags};
use linux_embedded_hal::{CdevPin, Delay, SpidevDevice};
use spidev::{SpiModeFlags, Spidev, SpidevOptions};

wasmtime::component::bindgen!({
    path: "../guest/wit",
    world: "app", // Make sure this matches your Pico host
});

#[derive(Deserialize)]
struct HostPolicy {
    spi: BTreeMap<String, SpiPolicy>,
    gpio: BTreeMap<String, GpioPolicy>,
}

#[derive(Deserialize)]
struct SpiPolicy {
    path: String,
    frequency: u32,
    mode: u8,
}

#[derive(Deserialize)]
struct GpioPolicy {
    pin: u32, // Changed to u32 for the CDEV API
    initial: String,
}

struct HostState {
    spi_ctx: SpiCtx,
    gpio_ctx: GpioCtx,
    delay_ctx: DelayCtx,
}

impl my::debug::logging::Host for HostState {
    fn log(&mut self, msg: String) {
        println!("[Guest Log] {}", msg);
    }
}

impl SpiView for HostState {
    fn spi_ctx(&mut self) -> &mut SpiCtx {
        &mut self.spi_ctx
    }
}
impl GpioView for HostState {
    fn gpio_ctx(&mut self) -> &mut GpioCtx {
        &mut self.gpio_ctx
    }
}
impl DelayView for HostState {
    fn delay_ctx(&mut self) -> &mut DelayCtx {
        &mut self.delay_ctx
    }
}

#[derive(Parser, Debug)]
#[command(author, version, about)]
pub struct HostArguments {
    #[arg(index = 1)]
    pub component_path: String,

    #[arg(long = "policy-file")]
    pub policy_file: String,
}

fn main() -> anyhow::Result<()> {
    let args = HostArguments::parse();

    // 1. Parse Policy TOML (with context)
    let policy_content = fs::read_to_string(&args.policy_file).with_context(|| {
        format!(
            "Failed to find or read policy file at '{}'",
            args.policy_file
        )
    })?;
    let policy: HostPolicy = toml::from_str(&policy_content)
        .with_context(|| format!("Failed to parse TOML in policy file '{}'", args.policy_file))?;

    // 2. Setup Linux SPI Devices based on policy
    let mut spi_hardware: Vec<(String, Box<dyn spi::ErasedSpiDevice + Send + 'static>)> =
        Vec::new();

    for (name, config) in policy.spi {
        let mut dev = Spidev::open(&config.path)
            .with_context(|| format!("Failed to open SPI device '{}' at path '{}'. Does this device exist on your machine?", name, config.path))?;

        let mode = match config.mode {
            0 => SpiModeFlags::SPI_MODE_0,
            1 => SpiModeFlags::SPI_MODE_1,
            2 => SpiModeFlags::SPI_MODE_2,
            3 => SpiModeFlags::SPI_MODE_3,
            _ => SpiModeFlags::SPI_MODE_0,
        };

        let options = SpidevOptions::new()
            .bits_per_word(8)
            .max_speed_hz(config.frequency)
            .mode(mode)
            .build();

        dev.configure(&options)
            .with_context(|| format!("Failed to configure SPI device '{}'", name))?;

        let spi_device = SpidevDevice(dev);
        spi_hardware.push((name, Box::new(spi_device)));
    }

    // 3. Setup Linux GPIO Devices based on policy (Modern CDEV API)
    let mut gpio_pins: BTreeMap<String, Box<dyn gpio::ErasedOutputPin + Send + 'static>> =
        BTreeMap::new();

    // Open the GPIO chip. On Pi 4 it is usually gpiochip4, on older ones it might be 0.
    let mut chip = Chip::new("/dev/gpiochip4")
        .or_else(|_| Chip::new("/dev/gpiochip0"))
        .with_context(|| "Failed to open GPIO chip. Are you running on a Raspberry Pi?")?;

    for (name, config) in policy.gpio {
        // Grab the specific pin line from the chip
        let line = chip
            .get_line(config.pin)
            .with_context(|| format!("Failed to find GPIO pin {}", config.pin))?;

        let default_val = if config.initial == "High" { 1 } else { 0 };

        // Request exclusive access to the pin as an output
        let handle = line
            .request(LineRequestFlags::OUTPUT, default_val, "wasm-host")
            .with_context(|| {
                format!(
                    "Failed to request GPIO pin {}. Is it already in use?",
                    config.pin
                )
            })?;

        // Wrap it in the embedded-hal trait
        let pin = CdevPin::new(handle)
            .with_context(|| format!("Failed to initialize CdevPin for {}", config.pin))?;

        gpio_pins.insert(name, Box::new(pin));
    }

    // 4. Initialize Wasmtime HostState using your custom Contexts
    let state = HostState {
        spi_ctx: SpiCtx {
            table: ResourceTable::new(),
            hardware: spi_hardware,
        },
        gpio_ctx: GpioCtx { pins: gpio_pins },
        delay_ctx: DelayCtx {
            delay: Box::new(Delay),
        },
    };

    // 5. Wasmtime Setup
    let mut config = Config::new();
    config.wasm_component_model(true);
    let engine = Engine::new(&config)?;
    let mut linker = Linker::new(&engine);

    // Bind your custom libraries to the linker
    spi::add_to_linker(&mut linker)?;
    gpio::add_to_linker(&mut linker)?;
    delay::add_to_linker(&mut linker)?;
    my::debug::logging::add_to_linker::<HostState, HasSelf<HostState>>(&mut linker, |state| state)?;

    let mut store = Store::new(&engine, state);

    // Load component (with context)
    let component = Component::from_file(&engine, &args.component_path)
        .with_context(|| format!("Failed to find Wasm component at '{}'", args.component_path))?;

    println!("Instantiating component...");
    let app = App::instantiate(&mut store, &component, &linker)?;

    println!("Calling guest run()...");
    app.call_run(&mut store)?;
    println!("Guest finished.");

    Ok(())
}
