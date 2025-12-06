use anyhow::{Context, Result};
use clap::Parser;
use linux_embedded_hal::spidev::SpidevOptions;
use linux_embedded_hal::{SpidevDevice, spidev};
use wasmtime::component::ResourceTable;
use wasmtime::component::{Component, HasSelf, Linker, Resource};
use wasmtime::{Config, Engine, Store};
use wasmtime_wasi::WasiCtxBuilder;
use wasmtime_wasi::p2::add_to_linker_sync;

mod argument_parser;
mod mock_spi;
mod spi_impl;
mod spi_trait;

use crate::argument_parser::HostArguments;
use crate::spi_trait::{SpiResource, WasiSpiDevice};
use mock_spi::MockSpiDevice;
use spi_impl::wasi::spi::spi as spi_bindings;
use spi_impl::{App, HostState};

fn main() -> Result<()> {
    let args = HostArguments::parse();
    let guest_path = args.component_path;

    let mut state = HostState {
        ctx: WasiCtxBuilder::new().inherit_stdio().build(),
        table: ResourceTable::new(),
    };

    let mut guest_device_rep: u32 = 0;

    for config in args.devices {
        println!(
            "Initializing SPI Device: {} -> {}",
            config.physical_path, config.virtual_name
        );

        let device_box: Box<dyn WasiSpiDevice> = if config.physical_path == "mock" {
            Box::new(MockSpiDevice)
        } else {
            let mut spi = SpidevDevice::open(&config.physical_path)
                .with_context(|| format!("Failed to open {}", config.physical_path))?;

            let options = SpidevOptions::new()
                .bits_per_word(config.bits_per_word)
                .max_speed_hz(config.max_speed_hz)
                .lsb_first(config.lsb_first)
                .mode(spidev::SpiModeFlags::from_bits_truncate(config.mode.into()))
                .build();

            spi.configure(&options)?;

            Box::new(spi)
        };

        let resource = SpiResource { device: device_box };

        let handle = state.table.push(resource)?;

        guest_device_rep = handle.rep();
    }

    let mut config = Config::new();
    config.wasm_component_model(true);
    let engine = Engine::new(&config)?;
    let mut linker = Linker::new(&engine);

    add_to_linker_sync(&mut linker)?;

    spi_bindings::add_to_linker::<HostState, HasSelf<HostState>>(
        &mut linker,
        |state: &mut HostState| state,
    )?;

    let run_handle = Resource::<spi_bindings::SpiDevice>::new_own(guest_device_rep);

    let mut store = Store::new(&engine, state);
    let component = Component::from_file(&engine, guest_path)?;
    let instance = App::instantiate(&mut store, &component, &linker)?;

    println!("Host: Calling guest run()...");
    instance.call_run(&mut store, run_handle)?;
    println!("Host: Guest finished.");

    Ok(())
}
