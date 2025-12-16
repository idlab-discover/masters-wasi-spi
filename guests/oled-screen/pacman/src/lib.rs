use wit_bindgen::generate;

generate!({
    path: "wit",
    world: "app",
    with: { "my:pmod-oled-driver/graphics": generate }
});

use crate::my::pmod_oled_driver::graphics::{Display, DisplayError, PixelColor};

struct PacmanApp;

impl Guest for PacmanApp {
    fn run() {
        let display = Display::new();

        display.on().expect("Failed to turn on screen");

        let w = display.width() as i32;
        let mut x = 0;
        let mut mouth_open = true;
        let mut frame = 0;

        loop {
            display.clear().unwrap();

            for dot_x in (10..120).step_by(15) {
                if dot_x > x + 5 {
                    safe_draw(&display, dot_x, 16);
                    safe_draw(&display, dot_x + 1, 16);
                    safe_draw(&display, dot_x, 17);
                    safe_draw(&display, dot_x + 1, 17);
                }
            }

            draw_pacman(&display, x, 16, 10, mouth_open);

            display.present().unwrap();

            x += 2;
            frame += 1;
            if x > w + 15 {
                x = -15;
            }
            if frame % 4 == 0 {
                mouth_open = !mouth_open;
            }

            display.delay_ms(16);
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

fn draw_pacman(d: &Display, cx: i32, cy: i32, r: i32, mouth: bool) {
    let r2 = r * r;
    for y in (cy - r)..=(cy + r) {
        for x in (cx - r)..=(cx + r) {
            if (x - cx).pow(2) + (y - cy).pow(2) <= r2 {
                if mouth && x > cx && (y - cy).abs() < (x - cx) {
                    continue;
                }

                safe_draw(d, x, y);
            }
        }
    }
}

export!(PacmanApp);
