#![no_std]
#![no_main]

extern crate alloc;

use alloc::string::String;
use embassy_executor::Spawner;
use embassy_rp::gpio::Output;
use embassy_rp::spi::{Blocking, Config as SpiConfig, Spi};
use embassy_time::{Delay, Instant};
use embedded_alloc::Heap;
use embedded_hal_bus::spi::ExclusiveDevice;
use wasmtime::component::{Component, HasSelf, Linker, ResourceTable};
use wasmtime::{Config, Engine, Store};
use {defmt_rtt as _, panic_probe as _};

use delay::{DelayCtx, DelayView};
use gpio::{GpioCtx, GpioView};
use pingpong::{Logger, SpiConfigurator, Timer, run_benchmark_matrix};
use spi::{ErasedSpiDevice, SpiCtx, SpiView};

wasmtime::component::bindgen!({
    path: "../guest/wit",
    world: "benchmark-app",
});

const HEAP_SIZE: usize = 470 * 1024;

#[global_allocator]
static HEAP: Heap = Heap::empty();

// Concrete types for our hardcoded Pico SPI
type PicoSpiBus = Spi<'static, embassy_rp::peripherals::SPI0, Blocking>;
type PicoSpiDevice = ExclusiveDevice<PicoSpiBus, Output<'static>, Delay>;

pub struct HostState {
    pub spi_ctx: SpiCtx,
    pub gpio_ctx: GpioCtx,
    pub delay_ctx: DelayCtx,
    // A backdoor to reconfigure the SPI baud rate from the Wasm runtime
    pub spi_raw_ptr: *mut PicoSpiDevice,
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

// --- WASI Imports for Wasm Benchmark ---
impl crate::my::timer::timer::Host for HostState {
    fn now_micros(&mut self) -> u64 {
        Instant::now().as_micros()
    }
}

impl crate::wasi::benchmark::bench_utils::Host for HostState {
    fn set_baud_rate(&mut self, baud: u32) {
        unsafe {
            if !self.spi_raw_ptr.is_null() {
                let mut config = SpiConfig::default();
                config.frequency = baud;
                // Safely reach into the type-erased bus and set the new hardware baud rate
                (*self.spi_raw_ptr).bus_mut().set_config(&config);
            }
        }
    }

    fn log(&mut self, msg: String) {
        defmt::info!("{=str}", msg.as_str());
    }
}

// --- Native Context Implementations ---
struct NativeBenchEnv;

impl Timer for NativeBenchEnv {
    type Instant = Instant;
    fn now(&self) -> Self::Instant {
        Instant::now()
    }
    fn elapsed_us(&self, start: Self::Instant) -> u64 {
        start.elapsed().as_micros()
    }
}

impl SpiConfigurator<PicoSpiDevice> for NativeBenchEnv {
    type Error = ();
    fn set_baud_rate(&mut self, spi: &mut PicoSpiDevice, baud: u32) -> Result<(), Self::Error> {
        let mut config = SpiConfig::default();
        config.frequency = baud;
        spi.bus_mut().set_config(&config);
        Ok(())
    }
}

impl Logger for NativeBenchEnv {
    fn log(&mut self, msg: &str) {
        defmt::info!("{=str}", msg);
    }
}

// --- Wasmtime TLS Hooks ---
static mut TLS_PTR: *mut u8 = core::ptr::null_mut();
#[unsafe(no_mangle)]
pub extern "C" fn wasmtime_tls_get() -> *mut u8 {
    unsafe { TLS_PTR }
}
#[unsafe(no_mangle)]
pub extern "C" fn wasmtime_tls_set(ptr: *mut u8) {
    unsafe {
        TLS_PTR = ptr;
    }
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

    {
        use core::mem::MaybeUninit;
        static mut HEAP_MEM: [MaybeUninit<u8>; HEAP_SIZE] = [MaybeUninit::uninit(); HEAP_SIZE];
        unsafe { HEAP.init(core::ptr::addr_of_mut!(HEAP_MEM) as usize, HEAP_SIZE) }
    }

    // Print CSV Header
    defmt::info!("Environment,BaudRate,Size_Bytes,TotalTime_us,AvgRTT_us,LoopbackValid");

    // =====================================
    // 0. Hardcode the SPI Setup
    // =====================================
    let mut spi_config = SpiConfig::default();
    spi_config.frequency = 125_000;
    let spi_bus = Spi::new_blocking(
        p.SPI0, p.PIN_18, // CLK
        p.PIN_19, // MOSI
        p.PIN_16, // MISO
        spi_config,
    );
    let cs = Output::new(p.PIN_17, embassy_rp::gpio::Level::High); // CS

    let mut spi_device = ExclusiveDevice::new(spi_bus, cs, Delay).unwrap();

    let tx_buf = alloc::vec![0xA5; 4096];
    let mut rx_buf = alloc::vec![0x00; 4096];

    // =====================================
    // 1. Run Native Context
    // =====================================
    {
        let timer_env = NativeBenchEnv;
        let mut config_env = NativeBenchEnv;
        let mut log_env = NativeBenchEnv;

        let _ = run_benchmark_matrix(
            &mut spi_device,
            &timer_env,
            &mut config_env,
            &mut log_env,
            &tx_buf,
            &mut rx_buf,
            "Native Pico",
        );
    }

    // =====================================
    // 2. Run Wasm Context via Pulley
    // =====================================
    {
        // Box the SPI device and get a stable pointer for the baud rate backdoor
        let mut boxed_spi = alloc::boxed::Box::new(spi_device);
        let spi_raw_ptr = boxed_spi.as_mut() as *mut PicoSpiDevice;

        let mut spi_hardware: alloc::vec::Vec<(
            String,
            alloc::boxed::Box<dyn ErasedSpiDevice + Send + 'static>,
        )> = alloc::vec::Vec::new();
        spi_hardware.push(("bench".into(), boxed_spi));

        let mut config = Config::new();
        config.target("pulley32").unwrap();
        config.wasm_component_model(true);
        config.gc_support(false);
        config.signals_based_traps(false);
        config.memory_init_cow(false);
        config.memory_guard_size(0);
        config.memory_reservation(0);
        config.max_wasm_stack(16 * 1024);
        config.memory_reservation_for_growth(0);

        let engine = Engine::new(&config).expect("Engine failed");

        let host_state = HostState {
            spi_ctx: SpiCtx {
                table: ResourceTable::new(),
                hardware: spi_hardware,
            },
            // CHANGED Vec::new() to BTreeMap::new() HERE
            gpio_ctx: GpioCtx {
                pins: alloc::collections::BTreeMap::new(),
            },
            delay_ctx: DelayCtx {
                delay: alloc::boxed::Box::new(Delay),
            },
            spi_raw_ptr,
        };

        let mut store = Store::new(&engine, host_state);
        let mut linker = Linker::new(&engine);

        spi::add_to_linker(&mut linker).unwrap();
        gpio::add_to_linker(&mut linker).unwrap();
        delay::add_to_linker(&mut linker).unwrap();
        crate::my::timer::timer::add_to_linker::<HostState, HasSelf<HostState>>(
            &mut linker,
            |state| state,
        )
        .unwrap();
        crate::wasi::benchmark::bench_utils::add_to_linker::<HostState, HasSelf<HostState>>(
            &mut linker,
            |state| state,
        )
        .unwrap();

        let guest_bytes = include_bytes!("benchmark_guest.pulley");
        let component = unsafe { Component::deserialize(&engine, guest_bytes) }.unwrap();

        let app = BenchmarkApp::instantiate(&mut store, &component, &linker).unwrap();

        // Starts the run matrix directly inside the guest
        let _ = app.call_run_pingpong(&mut store);
    }

    defmt::info!("--- Benchmark Matrix Complete ---");
}
