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
        // 1. Create the Driver Resource
        let display = Display::new();

        // 2. Turn on the hardware (Required!)
        display.on().expect("Failed to turn on screen");

        let width = display.width() as i32;
        let height = display.height() as i32;

        // 3. Animation State
        let mut x = 64.0; // Use floats for smoother velocity
        let mut y = 16.0;
        let mut dx = 1.5; // X Velocity
        let mut dy = 1.0; // Y Velocity
        let radius = 4;

        loop {
            // A. Clear Buffer
            display.clear().unwrap();

            // B. Draw Ball
            draw_circle(&display, x as i32, y as i32, radius);

            // C. Flush to Screen
            display.present().unwrap();

            // D. Physics Step
            x += dx;
            y += dy;

            // Bounce X (Left/Right Walls)
            // Check if outer edge of ball hits wall
            if (x + radius as f32) >= width as f32 || (x - radius as f32) <= 0.0 {
                dx = -dx;
                // Clamp to inside to prevent sticking
                if x < radius as f32 {
                    x = radius as f32;
                }
                if x > (width - radius) as f32 {
                    x = (width - radius) as f32;
                }
            }

            // Bounce Y (Top/Bottom Walls)
            if (y + radius as f32) >= height as f32 || (y - radius as f32) <= 0.0 {
                dy = -dy;
                if y < radius as f32 {
                    y = radius as f32;
                }
                if y > (height - radius) as f32 {
                    y = (height - radius) as f32;
                }
            }

            // E. Frame Delay
            display.delay_ms(20); // ~50 FPS
        }
    }
}

// --- HELPERS ---

// Wrapper to ignore OutOfBounds errors (clipping)
fn safe_draw(d: &Display, x: i32, y: i32) {
    match d.set_pixel(x, y, PixelColor::On) {
        Ok(_) => {}
        Err(DisplayError::OutOfBounds) => {} // Just ignore pixels off screen
        Err(e) => panic!("Screen Error: {:?}", e),
    }
}

// Simple Midpoint Circle Algorithm
fn draw_circle(d: &Display, cx: i32, cy: i32, r: i32) {
    let r2 = r * r;
    for y in (cy - r)..=(cy + r) {
        for x in (cx - r)..=(cx + r) {
            // Check if pixel is inside circle
            if (x - cx).pow(2) + (y - cy).pow(2) <= r2 {
                safe_draw(d, x, y);
            }
        }
    }
}

export!(DvdBounceApp);
