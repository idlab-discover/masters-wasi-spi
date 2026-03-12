#![no_std]
extern crate alloc;

use core::marker::PhantomData;
use wasmtime::component::{HasData, Linker};

wasmtime::component::bindgen!({
    path: "../../wit/delay.wit",
    world: "wasi-delay-host",
});

pub struct DelayCtx {
    pub delay: alloc::boxed::Box<dyn embedded_hal::delay::DelayNs + Send + 'static>,
}

pub trait DelayView {
    fn delay_ctx(&mut self) -> &mut DelayCtx;
}

pub struct DelayImpl<'a, T> {
    pub host: &'a mut T,
}

impl<'a, T: DelayView> wasi::delay::delay::Host for DelayImpl<'a, T> {
    fn delay_ms(&mut self, ms: u32) {
        self.host.delay_ctx().delay.delay_ms(ms);
    }
}

pub struct DelayBindingMarker<T>(PhantomData<T>);
impl<T: DelayView + 'static> HasData for DelayBindingMarker<T> {
    type Data<'a> = DelayImpl<'a, T>;
}
pub fn add_to_linker<T: DelayView + 'static>(linker: &mut Linker<T>) -> wasmtime::Result<()> {
    wasi::delay::delay::add_to_linker::<T, DelayBindingMarker<T>>(linker, |host| DelayImpl { host })
}
