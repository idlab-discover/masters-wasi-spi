use crate::wasi::delay::delay::delay_ms;
use crate::wasi::gpio::gpio::{Level, set_pin_state};
use crate::wasi::spi::spi::SpiDevice; // Removed open_device
use embedded_graphics::{pixelcolor::BinaryColor, prelude::*};

const WIDTH: u32 = 128;
const HEIGHT: u32 = 32;

const INIT_SEQUENCE: &[u8] = &[
    0xAE, 0x2E, 0xD5, 0x80, 0xA8, 0x1F, 0xD3, 0x00, 0x40, 0x8D, 0x14, 0x20, 0x00, 0xA1, 0xC8, 0xDA,
    0x02, 0x81, 0x8F, 0xD9, 0xF1, 0xDB, 0x40, 0xA4, 0xA6,
];

pub struct OledDisplay {
    spi: SpiDevice,
    buffer: [u8; 512],
    is_on: bool,
}

impl OledDisplay {
    // We now take ownership of the SpiDevice directly
    pub fn new(spi: SpiDevice) -> Self {
        crate::my::debug::logging::log("Initializing OLED Display...");

        Self {
            spi,
            buffer: [0u8; 512],
            is_on: false,
        }
    }

    pub fn on(&mut self) {
        if self.is_on {
            return;
        }

        set_pin_state("VBATC", Level::High);
        set_pin_state("VDDC", Level::High);
        delay_ms(100);
        set_pin_state("VDDC", Level::Low);
        delay_ms(100);
        set_pin_state("VBATC", Level::Low);
        delay_ms(100);

        set_pin_state("RES", Level::High);
        delay_ms(1);
        set_pin_state("RES", Level::Low);
        delay_ms(10);
        set_pin_state("RES", Level::High);

        for &c in INIT_SEQUENCE {
            self.send_cmd(c);
        }

        self.is_on = true;
        self.clear();
        self.present();
        self.send_cmd(0xAF); // Display ON
    }

    pub fn clear(&mut self) {
        self.buffer.fill(0);
    }

    pub fn present(&self) {
        if !self.is_on {
            return;
        }

        self.send_cmd(0x21); // Set column address
        self.send_cmd(0);
        self.send_cmd(127);
        self.send_cmd(0x22); // Set page address
        self.send_cmd(0);
        self.send_cmd(3);

        set_pin_state("DC", Level::High); // Data mode
        let _ = self.spi.write(&self.buffer);
    }

    fn send_cmd(&self, c: u8) {
        set_pin_state("DC", Level::Low); // Command mode
        let _ = self.spi.write(&[c]);
    }
}

// Implement embedded-graphics rendering capabilities for our display
impl DrawTarget for OledDisplay {
    type Color = BinaryColor;
    type Error = core::convert::Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        for Pixel(coord, color) in pixels.into_iter() {
            if coord.x >= 0 && coord.x < WIDTH as i32 && coord.y >= 0 && coord.y < HEIGHT as i32 {
                let idx = coord.x as usize + (coord.y as usize / 8) * 128;
                let bit = (coord.y % 8) as u8;
                match color {
                    BinaryColor::On => self.buffer[idx] |= 1 << bit,
                    BinaryColor::Off => self.buffer[idx] &= !(1 << bit),
                }
            }
        }
        Ok(())
    }
}

impl OriginDimensions for OledDisplay {
    fn size(&self) -> Size {
        Size::new(WIDTH, HEIGHT)
    }
}
