use anyhow::Result;
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

fn main() -> Result<()> {
    let guest_path = "../target/wasm32-wasip2/release/example_guest_component.wasm";

    let mut config = Config::new();
    config.wasm_component_model(true);
    let engine = Engine::new(&config)?;
    let mut linker = Linker::new(&engine);

    add_to_linker_sync(&mut linker)?;

    spi_bindings::add_to_linker::<HostState, HasSelf<HostState>>(&mut linker, |state: &mut HostState| state)?;

    let mut state = HostState {
        ctx: WasiCtxBuilder::new().inherit_stdio().build(),
        table: ResourceTable::new(),
    };

    let device1 = MockSpiDevice;
    let dev1_resource = state.table.push(device1)?;

    let device_handle = Resource::<spi_bindings::SpiDevice>::new_own(dev1_resource.rep());

    let mut store = Store::new(&engine, state);
    let component = Component::from_file(&engine, guest_path)?;
    let instance = App::instantiate(&mut store, &component, &linker)?;

    println!("Host: Calling guest run()...");
    instance.call_run(&mut store, device_handle)?;
    println!("Host: Guest finished.");

    Ok(())
}