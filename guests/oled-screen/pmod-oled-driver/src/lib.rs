use std::cell::{Cell, RefCell};
use wit_bindgen::generate;

generate!({
    path: "wit",
    world: "driver",
    with: {
        "wasi:spi/spi": generate,
        "wasi:gpio/digital@0.2.0": generate,
        "wasi:gpio/delay@0.2.0": generate,
        "wasi:gpio/general@0.2.0": generate,
        "wasi:gpio/poll@0.2.0": generate,
    }
});

use crate::exports::my::pmod_oled_driver::graphics::{
    DisplayError, Guest, GuestDisplay, PixelColor,
};
use crate::wasi::gpio::delay::delay_ms as host_delay_ms;
use crate::wasi::gpio::digital::{DigitalFlag, DigitalOutPin, PinState};
use crate::wasi::spi::spi::{Config, Mode, SpiDevice, get_device_names, open_device};

const WIDTH: u32 = 128;
const HEIGHT: u32 = 32;

const INIT_SEQUENCE: &[u8] = &[
    0xAE, 0x2E, 0xD5, 0x80, 0xA8, 0x1F, 0xD3, 0x00, 0x40, 0x8D, 0x14, 0x20, 0x00, 0xA1, 0xC8, 0xDA,
    0x02, 0x81, 0x8F, 0xD9, 0xF1, 0xDB, 0x40, 0xA4, 0xA6,
];

struct OledDriver;

impl Guest for OledDriver {
    type Display = Display;
}

pub struct Display {
    spi: SpiDevice,
    dc: DigitalOutPin,

    _res: DigitalOutPin,
    vbatc: DigitalOutPin,
    vddc: DigitalOutPin,

    buffer: RefCell<Vec<u8>>,
    is_on: Cell<bool>,
}

impl GuestDisplay for Display {
    fn new() -> Self {
        let names = get_device_names();

        let spi = open_device(&names[0]).expect("No SPI device found");
        spi.configure(Config {
            frequency: 8_000_000,
            mode: Mode::Mode0,
            lsb_first: false,
        })
        .unwrap();

        let flags_out = &[DigitalFlag::OUTPUT, DigitalFlag::ACTIVE_HIGH];
        let flags_low = &[DigitalFlag::OUTPUT, DigitalFlag::ACTIVE_LOW];

        let dc = DigitalOutPin::get("DC", flags_out).expect("DC pin");
        let res = DigitalOutPin::get("RES", flags_low).expect("RES pin");
        let vbatc = DigitalOutPin::get("VBATC", flags_low).expect("VBATC pin");
        let vddc = DigitalOutPin::get("VDDC", flags_low).expect("VDDC pin");

        Self {
            spi,
            dc,
            _res: res,
            vbatc,
            vddc,
            buffer: RefCell::new(vec![0u8; 512]),
            is_on: Cell::new(false),
        }
    }

    fn on(&self) -> Result<(), DisplayError> {
        if self.is_on.get() {
            return Ok(());
        }

        self.vbatc.set_state(PinState::Inactive).ok();
        self.vddc.set_state(PinState::Inactive).ok();
        host_delay_ms(100);
        self.vddc.set_state(PinState::Active).ok();
        host_delay_ms(100);
        self.vbatc.set_state(PinState::Active).ok();
        host_delay_ms(100);

        self._res.set_state(PinState::Inactive).ok();
        host_delay_ms(1);
        self._res.set_state(PinState::Active).ok();
        host_delay_ms(10);
        self._res.set_state(PinState::Inactive).ok();

        for &c in INIT_SEQUENCE {
            self.send_cmd(c)?;
        }

        self.is_on.set(true);

        self.clear()?;
        self.present()?;
        self.send_cmd(0xAF)?;

        Ok(())
    }

    fn off(&self) -> Result<(), DisplayError> {
        if !self.is_on.get() {
            return Ok(());
        }
        self.send_cmd(0xAE)?;
        self.is_on.set(false);
        Ok(())
    }

    fn width(&self) -> u32 {
        WIDTH
    }

    fn height(&self) -> u32 {
        HEIGHT
    }

    fn clear(&self) -> Result<(), DisplayError> {
        if !self.is_on.get() {
            return Err(DisplayError::DisplayOff);
        }
        self.buffer.borrow_mut().fill(0);
        Ok(())
    }

    fn set_pixel(&self, x: i32, y: i32, color: PixelColor) -> Result<(), DisplayError> {
        if !self.is_on.get() {
            return Err(DisplayError::DisplayOff);
        }

        if x < 0 || x >= WIDTH as i32 || y < 0 || y >= HEIGHT as i32 {
            return Err(DisplayError::OutOfBounds);
        }

        let idx = x as usize + (y as usize / 8) * 128;
        let bit = (y % 8) as u8;

        let mut buf = self.buffer.borrow_mut();
        match color {
            PixelColor::On => buf[idx] |= 1 << bit,
            PixelColor::Off => buf[idx] &= !(1 << bit),
        }
        Ok(())
    }

    fn present(&self) -> Result<(), DisplayError> {
        if !self.is_on.get() {
            return Err(DisplayError::DisplayOff);
        }

        self.send_cmd(0x21)?;
        self.send_cmd(0)?;
        self.send_cmd(127)?;
        self.send_cmd(0x22)?;
        self.send_cmd(0)?;
        self.send_cmd(3)?;

        self.dc
            .set_state(PinState::Active)
            .map_err(|_| DisplayError::HardwareError)?;

        self.spi
            .write(&self.buffer.borrow())
            .map_err(|_| DisplayError::HardwareError)?;

        Ok(())
    }

    fn delay_ms(&self, ms: u32) {
        host_delay_ms(ms as u64);
    }
}

impl Display {
    fn send_cmd(&self, c: u8) -> Result<(), DisplayError> {
        self.dc
            .set_state(PinState::Inactive)
            .map_err(|_| DisplayError::HardwareError)?;
        self.spi
            .write(&[c])
            .map_err(|_| DisplayError::HardwareError)?;
        Ok(())
    }
}

export!(OledDriver);
