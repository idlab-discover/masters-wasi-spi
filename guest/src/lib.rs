wit_bindgen::generate!({
    path: "wit",
    world: "app", // Replace with your actual world name if different
    generate_all
});

extern crate alloc;

mod bme280;
mod oled;

use crate::bme280::Bme280;
use crate::my::debug::logging::log;
use crate::oled::OledDisplay;
use crate::wasi::delay::delay::delay_ms;

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

        let mut display = OledDisplay::new("screen");
        display.on();

        let sensor = Bme280::new("sensor");
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
