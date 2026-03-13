wit_bindgen::generate!({
    path: "wit",
    world: "app",
    generate_all
});

extern crate alloc;

use crate::my::debug::logging::log;
use alloc::format;
use wasi_embedded_hal::{WasiDelay, WasiOutputPin, WasiSpiDevice};

use embedded_hal::digital::OutputPin;

use bme280::spi::BME280;
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
        log("System Starting...");
        let mut delay = WasiDelay;

        let mut oled_res = WasiOutputPin::new("RES");
        let mut oled_vbatc = WasiOutputPin::new("VBATC");
        let mut oled_vddc = WasiOutputPin::new("VDDC");
        let oled_dc = WasiOutputPin::new("DC");

        let _ = oled_vddc.set_low();

        let _ = oled_res.set_low();
        embedded_hal::delay::DelayNs::delay_us(&mut delay, 10);
        let _ = oled_res.set_high();

        let screen_spi = WasiSpiDevice::open("screen").expect("Failed screen SPI");
        let sensor_spi = WasiSpiDevice::open("sensor").expect("Failed sensor SPI");

        let interface = SPIInterface::new(screen_spi, oled_dc);
        let mut display = Ssd1306::new(interface, DisplaySize128x32, DisplayRotation::Rotate0)
            .into_buffered_graphics_mode();

        display.init().expect("SSD1306 software init failed");

        let _ = oled_vbatc.set_low();
        embedded_hal::delay::DelayNs::delay_ms(&mut delay, 100);

        log("Display and Sensor Ready.");

        let mut bme280 = BME280::new(sensor_spi).expect("Sensor setup failed");
        bme280.init(&mut delay).expect("Sensor calibration failed");

        let text_style = MonoTextStyle::new(&FONT_6X10, BinaryColor::On);

        loop {
            if let Ok(m) = bme280.measure(&mut delay) {
                let temp = format!("Temp: {:.1} C", m.temperature);
                let hum = format!("Hum:  {:.1} %", m.humidity);

                display.clear_buffer();
                let _ = Text::new(&temp, Point::new(0, 10), text_style).draw(&mut display);
                let _ = Text::new(&hum, Point::new(0, 24), text_style).draw(&mut display);
                let _ = display.flush();
            }
            embedded_hal::delay::DelayNs::delay_ms(&mut delay, 2000);
        }
    }
}

export!(MainApp);
