#![no_std]
extern crate alloc;

use wasmtime::component::Linker;

wasmtime::component::bindgen!({
    path: "../../../wit/delay.wit",
    world: "wasi-delay-host",
});

pub struct DelayCtx {
    pub delay: alloc::boxed::Box<dyn embedded_hal::delay::DelayNs + Send + 'static>,
}

pub trait DelayView {
    fn delay_ctx(&mut self) -> &mut DelayCtx;
}

impl wasi::delay::delay::Host for DelayCtx {
    fn delay_ms(&mut self, ms: u32) {
        self.delay.delay_ms(ms);
    }
}

pub fn add_to_linker<T: DelayView + 'static>(linker: &mut Linker<T>) -> wasmtime::Result<()> {
    wasi::delay::delay::add_to_linker::<T, wasmtime::component::HasSelf<DelayCtx>>(linker, |host| {
        host.delay_ctx()
    })
}
