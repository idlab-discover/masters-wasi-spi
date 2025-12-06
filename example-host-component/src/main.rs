use anyhow::Result;
use clap::Parser;
use linux_embedded_hal::spidev;
use linux_embedded_hal::spidev::{Spidev, SpidevOptions};
use wasmtime::{Config, Engine, Store};
use wasmtime::component::{Component, Linker, Resource, HasSelf};
use wasmtime_wasi::p2::add_to_linker_sync;
use wasmtime_wasi::WasiCtxBuilder;
use wasmtime::component::ResourceTable;

mod mock_spi;
mod spi_impl;
mod argument_parser;

use mock_spi::MockSpiDevice;
use spi_impl::{HostState, App};
use spi_impl::wasi::spi::spi as spi_bindings;
use crate::argument_parser::HostArguments;

fn main() -> Result<()> {
    let args = HostArguments::parse();
    let guest_path = args.component_path;

    let mut state = HostState {
        ctx: WasiCtxBuilder::new().inherit_stdio().build(),
        table: ResourceTable::new(),
    };

    let mut device_handle;


    for config in args.devices {
        println!("Initializing SPI Device: {} -> {}", config.physical_path, config.virtual_name);

        // A. Open Physical Device
        // let mut spidev = Spidev::open(&config.physical_path)?;
        //
        // // B. Apply Config
        // let options = SpidevOptions::new()
        //     .bits_per_word(config.bits_per_word)
        //     .max_speed_hz(config.max_speed_hz)
        //     .lsb_first(config.lsb_first)
        //     .mode(spidev::SpiModeFlags::from_bits_truncate(config.mode.into()))
        //     .build();
        // spidev.configure(&options)?;

        device_handle = state.table.push(MockSpiDevice)?;
    }

    let mut config = Config::new();
    config.wasm_component_model(true);
    let engine = Engine::new(&config)?;
    let mut linker = Linker::new(&engine);

    add_to_linker_sync(&mut linker)?;

    spi_bindings::add_to_linker::<HostState, HasSelf<HostState>>(&mut linker, |state: &mut HostState| state)?;

    let device_handle2 = Resource::<spi_bindings::SpiDevice>::new_own(device_handle.rep());

    let mut store = Store::new(&engine, state);
    let component = Component::from_file(&engine, guest_path)?;
    let instance = App::instantiate(&mut store, &component, &linker)?;

    println!("Host: Calling guest run()...");
    instance.call_run(&mut store, device_handle2)?;
    println!("Host: Guest finished.");

    Ok(())
}