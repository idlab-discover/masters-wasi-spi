#![no_std]
#![no_main]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::ToString;
use defmt::info;
use embassy_executor::Spawner;
use embassy_rp::gpio::{Level, Output};
use embassy_rp::spi::{Config as RpSpiConfig, Phase, Polarity, Spi};
use embedded_alloc::Heap;
use {defmt_rtt as _, panic_probe as _};

use wasmtime::component::{Component, HasSelf, Linker, ResourceTable};
use wasmtime::{Config, Engine, Store};

// Import contexts and views
use delay::{DelayCtx, DelayView};
use gpio::{GpioCtx, GpioView};
use spi::{SpiCtx, SpiView};

// Update this path if you changed your combined component's wit world location
wasmtime::component::bindgen!({
    path: "../guest/wit",
    world: "app", // Replace with your new combined world name (e.g., "app")
});

const HEAP_SIZE: usize = 470 * 1024;

#[global_allocator]
static HEAP: Heap = Heap::empty();

// --- Host State ---
pub struct HostState {
    pub spi_ctx: SpiCtx,
    pub gpio_ctx: GpioCtx,
    pub delay_ctx: DelayCtx,
}

impl my::debug::logging::Host for HostState {
    fn log(&mut self, msg: alloc::string::String) {
        defmt::info!("[Guest] {}", msg.as_str());
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

    // Initialize Heap
    {
        use core::mem::MaybeUninit;
        static mut HEAP_MEM: [MaybeUninit<u8>; HEAP_SIZE] = [MaybeUninit::uninit(); HEAP_SIZE];
        unsafe { HEAP.init(core::ptr::addr_of_mut!(HEAP_MEM) as usize, HEAP_SIZE) }
    }

    info!("Heap initialized.");

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

    // ==========================================
    // --- 1. Initialize SPI0 (BME280 Sensor) ---
    // ==========================================
    let mut spi0_config = RpSpiConfig::default();
    spi0_config.frequency = 1_000_000;
    spi0_config.polarity = Polarity::IdleLow;
    spi0_config.phase = Phase::CaptureOnFirstTransition;

    let spi0_driver = Spi::new_blocking(p.SPI0, p.PIN_18, p.PIN_19, p.PIN_16, spi0_config);
    let cs0_pin = Output::new(p.PIN_17, Level::High);

    // ==========================================
    // --- 2. Initialize SPI1 (OLED Screen) -----
    // ==========================================
    let mut spi1_config = RpSpiConfig::default();
    spi1_config.frequency = 8_000_000;
    spi1_config.polarity = Polarity::IdleLow;
    spi1_config.phase = Phase::CaptureOnFirstTransition;

    // PIN_12 is unused by OLED but mapped to satisfy embassy's MISO requirement
    let spi1_driver = Spi::new_blocking(p.SPI1, p.PIN_10, p.PIN_11, p.PIN_12, spi1_config);
    let cs1_pin = Output::new(p.PIN_13, Level::High);

    // ==========================================
    // --- 3. Initialize OLED GPIO Pins ---------
    // ==========================================
    let mut gpio_map = BTreeMap::new();
    // Insert OLED pins (initial states match active-low power logic)
    gpio_map.insert("DC".to_string(), Output::new(p.PIN_2, Level::Low));
    gpio_map.insert("RES".to_string(), Output::new(p.PIN_3, Level::High));
    gpio_map.insert("VBATC".to_string(), Output::new(p.PIN_4, Level::High));
    gpio_map.insert("VDDC".to_string(), Output::new(p.PIN_5, Level::High));

    // Construct the Wasmtime host state
    let host_state = HostState {
        spi_ctx: SpiCtx {
            table: ResourceTable::new(),
            spi0: spi0_driver,
            cs0: cs0_pin,
            spi1: spi1_driver,
            cs1: cs1_pin,
        },
        gpio_ctx: GpioCtx {
            pins: gpio_map, // The mapped pins are seamlessly handed to the guest
        },
        delay_ctx: DelayCtx {},
    };

    let mut store = Store::new(&engine, host_state);
    let mut linker = Linker::new(&engine);

    spi::add_to_linker(&mut linker).unwrap();
    gpio::add_to_linker(&mut linker).unwrap();
    delay::add_to_linker(&mut linker).unwrap();
    my::debug::logging::add_to_linker::<HostState, HasSelf<HostState>>(&mut linker, |state| state)
        .unwrap();

    let guest_bytes = include_bytes!("guest.pulley");
    info!(
        "Deserializing component (Size: {} bytes)...",
        guest_bytes.len()
    );

    let component = unsafe { Component::deserialize(&engine, guest_bytes) }.unwrap();

    info!("Instantiating...");
    // Update `App` to whatever bindgen named your combined component's main export
    let app = App::instantiate(&mut store, &component, &linker).unwrap();

    info!("Starting guest...");
    // Make sure this matches the combined app's run function call
    app.call_run(&mut store).unwrap();
}
