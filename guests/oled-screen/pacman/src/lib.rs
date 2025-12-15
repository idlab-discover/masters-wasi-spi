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
        // 1. Create the Driver Resource
        let display = Display::new();

        // 2. Turn on the hardware (Required!)
        display.on().expect("Failed to turn on screen");

        let w = display.width() as i32;
        let mut x = 0;
        let mut mouth_open = true;
        let mut frame = 0;

        loop {
            // Draw Frame
            // We unwrap clear() because if the display dies, the game should crash
            display.clear().unwrap();

            // Draw Food
            for dot_x in (10..120).step_by(15) {
                if dot_x > x + 5 {
                    safe_draw(&display, dot_x, 16);
                    safe_draw(&display, dot_x + 1, 16);
                    safe_draw(&display, dot_x, 17);
                    safe_draw(&display, dot_x + 1, 17);
                }
            }

            // Draw Pacman
            draw_pacman(&display, x, 16, 10, mouth_open);

            // Flush to Screen
            display.present().unwrap();

            // Game Logic
            x += 2;
            frame += 1;
            if x > w + 15 {
                x = -15;
            }
            if frame % 4 == 0 {
                mouth_open = !mouth_open;
            }

            // Frame Rate Control
            display.delay_ms(16);
        }
    }
}

// Helper to ignore OutOfBounds errors (clipping)
fn safe_draw(d: &Display, x: i32, y: i32) {
    match d.set_pixel(x, y, PixelColor::On) {
        Ok(_) => {}
        Err(DisplayError::OutOfBounds) => {} // Just ignore pixels off screen
        Err(e) => panic!("Screen Error: {:?}", e),
    }
}

fn draw_pacman(d: &Display, cx: i32, cy: i32, r: i32, mouth: bool) {
    let r2 = r * r;
    for y in (cy - r)..=(cy + r) {
        for x in (cx - r)..=(cx + r) {
            // Circle Equation: (x-cx)^2 + (y-cy)^2 <= r^2
            if (x - cx).pow(2) + (y - cy).pow(2) <= r2 {
                // Mouth Logic: Wedge pointing right
                if mouth && x > cx && (y - cy).abs() < (x - cx) {
                    continue;
                }

                safe_draw(d, x, y);
            }
        }
    }
}

export!(PacmanApp);
