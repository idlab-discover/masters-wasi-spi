wit_bindgen::generate!({
    path: "wit",
    world: "app",
    generate_all
});

extern crate alloc;

mod bme280;
mod oled;

use crate::bme280::Bme280;
use crate::my::debug::logging::log;
use crate::oled::OledDisplay;
use crate::wasi::delay::delay::delay_ms;
use crate::wasi::spi::spi::get_devices; // Import the new function

use alloc::collections::BTreeMap;
use alloc::format;
use embedded_graphics::{
    mono_font::{MonoTextStyle, ascii::FONT_6X10},
    pixelcolor::BinaryColor,
    prelude::*,
    text::Text,
};

struct MainApp;

impl Guest for MainApp {
    fn run() {
        log("Starting unified Temp Display Component!");

        // 1. Fetch all capabilities granted by the host exactly once
        let mut spi_hardware: BTreeMap<String, crate::wasi::spi::spi::SpiDevice> = get_devices()
            .expect("Failed to fetch SPI devices from host")
            .into_iter()
            .collect();

        // 2. Extract strictly what we need.
        // If the host policy.toml is wrong, the Wasm module panics securely right here!
        let screen_spi = spi_hardware
            .remove("screen")
            .expect("CRITICAL: Host failed to inject 'screen' SPI device!");
        let sensor_spi = spi_hardware
            .remove("sensor")
            .expect("CRITICAL: Host failed to inject 'sensor' SPI device!");

        // 3. Hand the physical capabilities over to our drivers
        let mut display = OledDisplay::new(screen_spi);
        display.on();

        let sensor = Bme280::new(sensor_spi);

        let text_style = MonoTextStyle::new(&FONT_6X10, BinaryColor::On);

        loop {
            let (temp, humidity) = sensor.read();
            let temp_str = format!("Temp: {:.1} C", temp);
            let hum_str = format!("Hum:  {:.1} %", humidity);

            log(&temp_str); // Fallback debug logging

            display.clear();

            Text::new(&temp_str, Point::new(0, 10), text_style)
                .draw(&mut display)
                .unwrap();

            Text::new(&hum_str, Point::new(0, 24), text_style)
                .draw(&mut display)
                .unwrap();

            display.present();
            delay_ms(2000);
        }
    }
}

export!(MainApp);
