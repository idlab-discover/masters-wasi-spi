use wit_bindgen::generate;

generate!({
    path: "wit",
    world: "app",
    with: { "my:pmod-oled-driver/graphics": generate }
});

use crate::my::pmod_oled_driver::graphics::{Display, DisplayError, PixelColor};

struct DvdBounceApp;

impl Guest for DvdBounceApp {
    fn run() {
        let display = Display::new();

        display.on().expect("Failed to turn on screen");

        let width = display.width() as i32;
        let height = display.height() as i32;

        let mut x = 64.0;
        let mut y = 16.0;
        let mut dx = 1.5;
        let mut dy = 1.0;
        let radius = 4;

        loop {
            display.clear().unwrap();

            draw_circle(&display, x as i32, y as i32, radius);

            display.present().unwrap();

            x += dx;
            y += dy;

            if (x + radius as f32) >= width as f32 || (x - radius as f32) <= 0.0 {
                dx = -dx;

                if x < radius as f32 {
                    x = radius as f32;
                }
                if x > (width - radius) as f32 {
                    x = (width - radius) as f32;
                }
            }

            if (y + radius as f32) >= height as f32 || (y - radius as f32) <= 0.0 {
                dy = -dy;
                if y < radius as f32 {
                    y = radius as f32;
                }
                if y > (height - radius) as f32 {
                    y = (height - radius) as f32;
                }
            }

            display.delay_ms(20);
        }
    }
}

fn safe_draw(d: &Display, x: i32, y: i32) {
    match d.set_pixel(x, y, PixelColor::On) {
        Ok(_) => {}
        Err(DisplayError::OutOfBounds) => {}
        Err(e) => panic!("Screen Error: {:?}", e),
    }
}

fn draw_circle(d: &Display, cx: i32, cy: i32, r: i32) {
    let r2 = r * r;
    for y in (cy - r)..=(cy + r) {
        for x in (cx - r)..=(cx + r) {
            if (x - cx).pow(2) + (y - cy).pow(2) <= r2 {
                safe_draw(d, x, y);
            }
        }
    }
}

export!(DvdBounceApp);
