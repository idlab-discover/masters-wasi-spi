wit_bindgen::generate!({
    path: "wit",
    world: "app",
    generate_all
});

extern crate alloc;

use crate::my::debug::logging::log;
use alloc::format;
use alloc::vec::Vec;
use wasi_embedded_hal::{WasiDelay, WasiOutputPin, WasiSpiDevice};

use display_interface_spi::SPIInterface;
use embedded_hal::digital::OutputPin;
use ssd1306::{Ssd1306, prelude::*};

use embedded_graphics::{
    mono_font::{MonoTextStyle, ascii::FONT_6X10},
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{Circle, PrimitiveStyle, PrimitiveStyleBuilder, Rectangle},
    text::Text,
};

// A simple particle for our physics simulation
struct Particle {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    r: f32,
}

struct MainApp;

impl Guest for MainApp {
    fn run() {
        log("System Starting: 0G Physics Demo");
        let mut delay = WasiDelay;

        let mut oled_res = WasiOutputPin::new("RES");
        let mut oled_vbatc = WasiOutputPin::new("VBATC");
        let mut oled_vddc = WasiOutputPin::new("VDDC");
        let oled_dc = WasiOutputPin::new("DC");

        // Screen bootup sequence
        let _ = oled_vddc.set_low();
        let _ = oled_res.set_low();
        embedded_hal::delay::DelayNs::delay_us(&mut delay, 10);
        let _ = oled_res.set_high();

        let screen_spi = WasiSpiDevice::open("screen").expect("Failed screen SPI");

        let mut display = Ssd1306::new(
            SPIInterface::new(screen_spi, oled_dc),
            DisplaySize128x32,
            DisplayRotation::Rotate0,
        )
        .into_buffered_graphics_mode();

        display.init().expect("SSD1306 software init failed");

        let _ = oled_vbatc.set_low();
        embedded_hal::delay::DelayNs::delay_ms(&mut delay, 100);

        log("Display Ready. Starting 0G physics loop.");

        let text_style = MonoTextStyle::new(&FONT_6X10, BinaryColor::On);
        let particle_style = PrimitiveStyle::with_stroke(BinaryColor::On, 1);

        let box_style = PrimitiveStyleBuilder::new()
            .fill_color(BinaryColor::Off)
            .stroke_color(BinaryColor::On)
            .stroke_width(1)
            .build();

        // Create 10 balls spread out across the screen, avoiding the top-left corner
        let mut particles = Vec::new();
        for i in 0..10 {
            particles.push(Particle {
                x: 60.0 + (i as f32 * 6.0),
                y: 18.0 + ((i % 3) as f32 * 4.0),
                vx: if i % 2 == 0 { 1.5 } else { -1.2 },
                vy: if i % 3 == 0 { 0.9 } else { -1.3 },
                r: 2.0,
            });
        }

        let mut frames: u32 = 0;
        let mut current_fps: u32 = 0;
        let mut last_time = crate::my::clock::time::now_ms();

        let box_h = 13.0;

        loop {
            // 1. Calculate true FPS first so we know how big the box needs to be
            frames += 1;
            let now = crate::my::clock::time::now_ms();
            if now - last_time >= 1000 {
                current_fps = frames;
                frames = 0;
                last_time = now;
            }

            let info = format!("FPS: {}", current_fps);
            // Dynamic width: 6 pixels per character + 5 pixels for padding
            let box_w = (info.len() as f32 * 6.0) + 5.0;

            display.clear_buffer();

            // 2. Move particles and handle wall & dynamic box collisions
            for p in particles.iter_mut() {
                p.x += p.vx;
                p.y += p.vy;

                // Screen boundaries (128x32)
                if p.x - p.r < 0.0 {
                    p.x = p.r;
                    p.vx = -p.vx;
                } else if p.x + p.r > 127.0 {
                    p.x = 127.0 - p.r;
                    p.vx = -p.vx;
                }

                if p.y - p.r < 0.0 {
                    p.y = p.r;
                    p.vy = -p.vy;
                } else if p.y + p.r > 31.0 {
                    p.y = 31.0 - p.r;
                    p.vy = -p.vy;
                }

                // FPS Box Collision (Top-Left corner) using dynamic width
                if p.x - p.r < box_w && p.y - p.r < box_h {
                    let overlap_x = box_w - (p.x - p.r);
                    let overlap_y = box_h - (p.y - p.r);

                    // Bounce off the side that has the smallest overlap
                    if overlap_x < overlap_y {
                        p.x = box_w + p.r;
                        p.vx = p.vx.abs();
                    } else {
                        p.y = box_h + p.r;
                        p.vy = p.vy.abs();
                    }
                }
            }

            // 3. Handle ball-to-ball collisions
            for i in 0..particles.len() {
                let (left, right) = particles.split_at_mut(i + 1);
                let p1 = &mut left[i];

                for p2 in right.iter_mut() {
                    let dx = p2.x - p1.x;
                    let dy = p2.y - p1.y;
                    let dist_sq = dx * dx + dy * dy;
                    let min_dist = p1.r + p2.r;

                    if dist_sq < min_dist * min_dist && dist_sq > 0.0001 {
                        let dist = dist_sq.sqrt();

                        let nx = dx / dist;
                        let ny = dy / dist;

                        let overlap = min_dist - dist;
                        p1.x -= nx * overlap * 0.5;
                        p1.y -= ny * overlap * 0.5;
                        p2.x += nx * overlap * 0.5;
                        p2.y += ny * overlap * 0.5;

                        let rvx = p2.vx - p1.vx;
                        let rvy = p2.vy - p1.vy;

                        let vel_along_normal = rvx * nx + rvy * ny;

                        if vel_along_normal > 0.0 {
                            continue;
                        }

                        let j = -2.0 * vel_along_normal;
                        let j = j / 2.0;

                        let impulse_x = j * nx;
                        let impulse_y = j * ny;

                        p1.vx -= impulse_x;
                        p1.vy -= impulse_y;
                        p2.vx += impulse_x;
                        p2.vy += impulse_y;
                    }
                }
            }

            // 4. Draw the particles
            for p in &particles {
                let _ = Circle::new(
                    Point::new((p.x - p.r) as i32, (p.y - p.r) as i32),
                    (p.r * 2.0) as u32,
                )
                .into_styled(particle_style)
                .draw(&mut display);
            }

            // 5. Draw the dynamic FPS box and text over everything else
            let _ = Rectangle::new(Point::new(0, 0), Size::new(box_w as u32, box_h as u32))
                .into_styled(box_style)
                .draw(&mut display);

            let _ = Text::new(&info, Point::new(3, 9), text_style).draw(&mut display);

            // Send the buffer to the physical screen
            let _ = display.flush();
        }
    }
}

export!(MainApp);
