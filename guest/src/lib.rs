// guest/src/lib.rs

wit_bindgen::generate!({
    path: "wit",
    world: "app",
    generate_all
});

extern crate alloc;

use crate::my::debug::logging::log;
use alloc::format;

// Our custom HAL library that wraps WASI imports into embedded-hal traits
use wasi_embedded_hal::{WasiDelay, WasiOutputPin, WasiSpiDevice};

// Community Drivers
use bme280::spi::BME280 as SpiBME280;
use display_interface_spi::SPIInterface;
use ssd1306::{Ssd1306, prelude::*};

use embedded_graphics::{
    mono_font::{MonoTextStyle, ascii::FONT_6X10},
    pixelcolor::BinaryColor,
    prelude::*,
    text::Text,
};

struct MainApp;

impl Guest for MainApp {
    fn run() {
        log("Starting unified Temp Display Component using community crates!");

        // 1. Open SPI devices. These strings MUST match labels in your policy.toml
        let screen_spi = match WasiSpiDevice::open("screen") {
            Ok(s) => s,
            Err(e) => {
                log(&format!("CRITICAL: Could not open 'screen' SPI: {:?}", e));
                return;
            }
        };

        let sensor_spi = match WasiSpiDevice::open("sensor") {
            Ok(s) => s,
            Err(e) => {
                log(&format!("CRITICAL: Could not open 'sensor' SPI: {:?}", e));
                return;
            }
        };

        // 2. Initialize GPIO and Delay
        let oled_dc = WasiOutputPin::new("oled_dc");
        let mut delay = WasiDelay;

        // 3. Initialize SSD1306 Display
        log("Initializing SSD1306 Display...");
        let display_interface = SPIInterface::new(screen_spi, oled_dc);
        let mut display = Ssd1306::new(
            display_interface,
            DisplaySize128x32,
            DisplayRotation::Rotate0,
        )
        .into_buffered_graphics_mode();

        if let Err(e) = display.init() {
            log(&format!("Display init failed: {:?}", e));
            // We continue anyway to see if the sensor works
        } else {
            log("Display initialized.");
        }

        // 4. Initialize BME280 Sensor
        log("Initializing BME280 Sensor...");
        // SpiBME280::new performs a Chip-ID check. If this fails, the wiring or CS is wrong.
        let mut bme280 = match SpiBME280::new(sensor_spi) {
            Ok(s) => s,
            Err(e) => {
                log(&format!("BME280 Hardware Error: {:?}", e));
                log("Check: SPI wiring, 3.3V power, and Chip Select (CS) logic.");
                return;
            }
        };

        if let Err(e) = bme280.init(&mut delay) {
            log(&format!("BME280 Calibration Error: {:?}", e));
            return;
        }

        let text_style = MonoTextStyle::new(&FONT_6X10, BinaryColor::On);
        log("System Ready. Entering measurement loop.");

        loop {
            match bme280.measure(&mut delay) {
                Ok(m) => {
                    let temp_str = format!("Temp: {:.1} C", m.temperature);
                    let hum_str = format!("Hum:  {:.1} %", m.humidity);

                    log(&temp_str);

                    display.clear_buffer();

                    let _ = Text::new(&temp_str, Point::new(0, 10), text_style).draw(&mut display);

                    let _ = Text::new(&hum_str, Point::new(0, 24), text_style).draw(&mut display);

                    let _ = display.flush();
                }
                Err(e) => {
                    log(&format!("Sensor Read Error: {:?}", e));
                }
            }

            // Wait 2 seconds
            embedded_hal::delay::DelayNs::delay_ms(&mut delay, 2000);
        }
    }
}

export!(MainApp);
