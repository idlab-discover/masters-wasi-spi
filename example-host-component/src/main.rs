use std::collections::HashMap;

use __internal::Vec;
use anyhow::Context;
use clap::Parser;
use embedded_hal::spi::{Operation, SpiDevice as HalSpiDevice};
use linux_embedded_hal::SpidevDevice;
use wasi::spi::spi as spi_bindings;
use wasmtime::{
    Config, Engine, Store,
    component::{
        __internal::{self},
        Component, HasSelf, Linker, Resource, bindgen,
    },
};
use wasmtime_wasi::{ResourceTable, WasiCtx, WasiCtxBuilder, WasiView, p2::add_to_linker_sync};

use crate::wasi::spi::spi::{NamedDevice, OperationResult};

mod argument_parser;

bindgen!({
    path: "../example-guest-component/wit",
    world: "app",
    with: {
        "wasi:spi/spi.spi-device": MySpiDevice
    }
});

pub struct MySpiDevice {
    device: SpidevDevice,
}

struct HostState {
    ctx: WasiCtx,
    table: ResourceTable,
    devices: HashMap<String, Resource<MySpiDevice>>,
}

impl WasiView for HostState {
    fn ctx(&mut self) -> wasmtime_wasi::WasiCtxView<'_> {
        wasmtime_wasi::WasiCtxView {
            ctx: &mut self.ctx,
            table: &mut self.table,
        }
    }
}

impl spi_bindings::Host for HostState {
    fn get_devices(&mut self) -> Vec<NamedDevice> {
        self.devices
            .iter()
            .map(|(name, device)| spi_bindings::NamedDevice {
                name: name.clone(),
                device: Resource::new_own(device.rep()),
            })
            .collect()
    }
}

impl spi_bindings::HostSpiDevice for HostState {
    fn configure(
        &mut self,
        handle: Resource<MySpiDevice>,
        config: spi_bindings::Config,
    ) -> Result<(), spi_bindings::Error> {
        let spi = self
            .table
            .get_mut(&handle)
            .map_err(|err| spi_bindings::Error::Other(err.to_string()))?;

        use linux_embedded_hal::spidev::{SpiModeFlags, SpidevOptions};

        let mode = match config.mode {
            spi_bindings::Mode::Mode0 => SpiModeFlags::SPI_MODE_0,
            spi_bindings::Mode::Mode1 => SpiModeFlags::SPI_MODE_1,
            spi_bindings::Mode::Mode2 => SpiModeFlags::SPI_MODE_2,
            spi_bindings::Mode::Mode3 => SpiModeFlags::SPI_MODE_3,
        };

        let options = SpidevOptions::new()
            .max_speed_hz(config.frequency)
            .mode(mode)
            .lsb_first(config.lsb_first)
            .build();

        spi.device
            .configure(&options)
            .map_err(|e| spi_bindings::Error::Other(e.to_string()))?;

        Ok(())
    }

    fn read(
        &mut self,
        handle: Resource<MySpiDevice>,
        len: u64,
    ) -> Result<Vec<u8>, spi_bindings::Error> {
        let my_spi_device = self
            .table
            .get_mut(&handle)
            .map_err(|err| spi_bindings::Error::Other(err.to_string()))?;

        let mut buf: Vec<u8> = vec![0u8; len as usize];

        my_spi_device
            .device
            .read(&mut buf)
            .map_err(|e| spi_bindings::Error::Other(e.to_string()))?;

        Ok(buf)
    }

    fn write(
        &mut self,
        handle: Resource<MySpiDevice>,
        data: Vec<u8>,
    ) -> Result<(), spi_bindings::Error> {
        let my_spi_device = self
            .table
            .get_mut(&handle)
            .map_err(|err| spi_bindings::Error::Other(err.to_string()))?;

        my_spi_device
            .device
            .write(&data)
            .map_err(|err| spi_bindings::Error::Other(err.to_string()))?;

        Ok(())
    }

    fn transfer(
        &mut self,
        handle: Resource<MySpiDevice>,
        data: Vec<u8>,
    ) -> Result<Vec<u8>, spi_bindings::Error> {
        let spi = self
            .table
            .get_mut(&handle)
            .map_err(|_| spi_bindings::Error::Other("Device not found".into()))?;

        let mut read_buf = vec![0u8; data.len()];

        spi.device
            .transfer(&mut read_buf, &data)
            .map_err(|e| spi_bindings::Error::Other(e.to_string()))?;

        Ok(read_buf)
    }

    fn transaction(
        &mut self,
        handle: Resource<spi_bindings::SpiDevice>,
        operations: Vec<spi_bindings::Operation>,
    ) -> Result<Vec<spi_bindings::OperationResult>, spi_bindings::Error> {
        let my_spi_device = self
            .table
            .get_mut(&handle)
            .map_err(|_| spi_bindings::Error::Other("Device not found".to_string()))?;

        enum TransactionBuffers {
            Read(Vec<u8>),
            Write(Vec<u8>),
            Transfer { read: Vec<u8>, write: Vec<u8> },
            DelayNs(u32),
        }

        let mut buffers: Vec<TransactionBuffers> = operations
            .into_iter()
            .map(|operation| match operation {
                spi_bindings::Operation::Read(len) => {
                    TransactionBuffers::Read(vec![0u8; len as usize])
                }

                spi_bindings::Operation::Transfer(write_buf) => TransactionBuffers::Transfer {
                    read: vec![0u8; write_buf.len()],
                    write: write_buf,
                },
                spi_bindings::Operation::Write(write_buf) => TransactionBuffers::Write(write_buf),
                spi_bindings::Operation::DelayNs(ns) => TransactionBuffers::DelayNs(ns),
            })
            .collect();

        let mut hal_operations: Vec<Operation<u8>> = buffers
            .iter_mut()
            .map(|buffer| match buffer {
                TransactionBuffers::Read(read_into_buf) => Operation::Read(read_into_buf),
                TransactionBuffers::Write(write_from_buf) => Operation::Write(write_from_buf),
                TransactionBuffers::Transfer { read, write } => Operation::Transfer(read, write),
                TransactionBuffers::DelayNs(ns) => Operation::DelayNs(*ns),
            })
            .collect();

        my_spi_device
            .device
            .transaction(&mut hal_operations)
            .map_err(|err| spi_bindings::Error::Other(err.to_string()))?;

        let results: Vec<OperationResult> = buffers
            .into_iter()
            .map(|buffer| match buffer {
                TransactionBuffers::Read(items) => OperationResult::Read(items),
                TransactionBuffers::Write(_) => OperationResult::Write,
                TransactionBuffers::Transfer { read, write: _ } => OperationResult::Transfer(read),
                TransactionBuffers::DelayNs(_) => OperationResult::Delay,
            })
            .collect();

        Ok(results)
    }

    fn drop(&mut self, rep: Resource<MySpiDevice>) -> wasmtime::Result<()> {
        let rep_id = rep.rep();
        let _device: MySpiDevice = self.table.delete(rep)?;
        self.devices.retain(|_, handle| handle.rep() != rep_id);
        Ok(())
    }
}

fn main() -> anyhow::Result<()> {
    let args = argument_parser::HostArguments::parse();
    let guest_path = args.component_path;
    let devices = args.devices;

    let mut state = HostState {
        ctx: WasiCtxBuilder::new().inherit_stdio().build(),
        table: ResourceTable::new(),
        devices: HashMap::new(),
    };

    for device in devices {
        let physical_path = device.physical_path;
        let virtual_name = device.virtual_name;

        let spi_device = MySpiDevice {
            device: SpidevDevice::open(&physical_path)
                .with_context(|| format!("Unable to open device at {}", physical_path))?,
        };

        let handle = state.table.push(spi_device)?;
        state.devices.insert(virtual_name, handle);
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

    let mut store = Store::new(&engine, state);
    let component = Component::from_file(&engine, guest_path)?;
    let instance = App::instantiate(&mut store, &component, &linker)?;

    println!("Host: Calling guest run()...");
    instance.call_run(&mut store)?;
    println!("Host: Guest finished.");

    Ok(())
}
