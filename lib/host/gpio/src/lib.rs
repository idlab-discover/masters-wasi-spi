#![no_std]
extern crate alloc;

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::String;
use embedded_hal::digital::OutputPin;
use wasmtime::component::Linker;

wasmtime::component::bindgen!({
    path: "../../../wit/gpio.wit",
    world: "wasi-gpio-host",
});

pub trait ErasedOutputPin {
    fn set_high(&mut self);
    fn set_low(&mut self);
}

impl<T: OutputPin> ErasedOutputPin for T {
    fn set_high(&mut self) {
        let _ = OutputPin::set_high(self);
    }
    fn set_low(&mut self) {
        let _ = OutputPin::set_low(self);
    }
}

pub struct GpioCtx {
    pub pins: BTreeMap<String, Box<dyn ErasedOutputPin + Send + 'static>>,
}

pub trait GpioView {
    fn gpio_ctx(&mut self) -> &mut GpioCtx;
}

impl wasi::gpio::gpio::Host for GpioCtx {
    fn set_pin_state(&mut self, label: String, level: wasi::gpio::gpio::Level) {
        if let Some(pin) = self.pins.get_mut(&label) {
            match level {
                wasi::gpio::gpio::Level::High => pin.set_high(),
                wasi::gpio::gpio::Level::Low => pin.set_low(),
            }
        }
    }
}

pub fn add_to_linker<T: GpioView + 'static>(linker: &mut Linker<T>) -> wasmtime::Result<()> {
    wasi::gpio::gpio::add_to_linker::<T, wasmtime::component::HasSelf<GpioCtx>>(linker, |host| {
        host.gpio_ctx()
    })
}
