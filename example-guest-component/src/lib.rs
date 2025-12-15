use wit_bindgen::generate;

generate!({
    path: "wit",
    world: "app",
    with: {
        "wasi:spi/spi": generate,
        "wasi:gpio/digital@0.2.0": generate,
        "wasi:gpio/delay@0.2.0": generate,
        "wasi:gpio/general@0.2.0": generate,
        "wasi:gpio/poll@0.2.0": generate,
    }
});

use crate::wasi::gpio::delay::delay_ms;
use crate::wasi::gpio::digital::{DigitalFlag, DigitalOutPin, PinState};
use crate::wasi::spi::spi::{SpiDevice, get_device_names, open_device};

// --- CONSTANTS ---
// 128x32 Configuration from Python Script
const INIT_SEQUENCE: &[u8] = &[
    0xAE, 0xD5, 0x80, 0xA8, 0x1F, 0xD3, 0x00, 0x40, 0x8D, 0x14, 0x20, 0x00, 0xA1, 0xC8, 0xDA, 0x02,
    0x81, 0x8F, 0xD9, 0xF1, 0xDB, 0x40, 0xA4, 0xA6, 0xAF,
];

const WIDTH: i32 = 128;
const HEIGHT: i32 = 32;

struct MyGuest;

impl Guest for MyGuest {
    fn run() {
        println!("--- [Guest] Starting Pacman ---");

        // 1. Setup SPI
        let device_names = get_device_names();
        if device_names.is_empty() {
            return;
        }
        let spi = open_device(&device_names[0]).expect("No SPI");

        // 2. Setup GPIO
        // DC is Active High (Low=Cmd, High=Data)
        let flags_dc = &[DigitalFlag::OUTPUT, DigitalFlag::ACTIVE_HIGH];
        // RES, VBATC, VDDC are Active Low in your Python script (Low = ON)
        let flags_pwr = &[DigitalFlag::OUTPUT, DigitalFlag::ACTIVE_LOW];

        let dc = DigitalOutPin::get("DC", flags_dc).expect("Err DC");
        let res = DigitalOutPin::get("RES", flags_pwr).expect("Err RES");
        let vbatc = DigitalOutPin::get("VBATC", flags_pwr).expect("Err VBATC");
        let vddc = DigitalOutPin::get("VDDC", flags_pwr).expect("Err VDDC");

        // 3. Init Display (Python Logic)
        // Start OFF (High physically -> Inactive logic because ACTIVE_LOW)
        vbatc.set_state(PinState::Inactive).unwrap();
        vddc.set_state(PinState::Inactive).unwrap();
        delay_ms(100);

        // Turn ON (Low physically -> Active logic)
        vddc.set_state(PinState::Active).unwrap(); // Logic ON
        delay_ms(100);
        vbatc.set_state(PinState::Active).unwrap(); // VBAT ON
        delay_ms(100);

        // Reset Pulse (High -> Low -> High)
        // With ACTIVE_LOW: Inactive(High) -> Active(Low) -> Inactive(High)
        res.set_state(PinState::Inactive).unwrap();
        delay_ms(1);
        res.set_state(PinState::Active).unwrap(); // Reset
        delay_ms(10);
        res.set_state(PinState::Inactive).unwrap(); // Operational

        // Send Config
        for &c in INIT_SEQUENCE {
            send_cmd(&spi, &dc, c);
        }

        // 4. Animation Loop
        let mut buffer = vec![0u8; 512];
        let mut pos_x: i32 = -15;
        let mut mouth_open = true;
        let mut frame_count = 0;

        println!("[Guest] Running Loop...");

        loop {
            // Clear Buffer
            buffer.fill(0);

            // Draw Dots
            draw_dots(&mut buffer, pos_x);

            // Draw Pacman
            draw_pacman(&mut buffer, pos_x, 16, 10, mouth_open);

            // Update Screen
            update_screen(&spi, &dc, &buffer);

            // Animation Logic
            // FIX 1: Move only 1 pixel at a time for smoothness
            pos_x += 1;
            frame_count += 1;

            // Toggle mouth every 10 frames (slower mouth animation)
            if frame_count % 10 == 0 {
                mouth_open = !mouth_open;
            }

            // Loop
            if pos_x > WIDTH + 15 {
                pos_x = -15;
            }

            // FIX 2: Increase delay to 33ms (~30 FPS)
            delay_ms(33);
        }
    }
}

// --- GRAPHICS HELPERS ---

fn set_pixel(buffer: &mut [u8], x: i32, y: i32) {
    if x >= 0 && x < WIDTH && y >= 0 && y < HEIGHT {
        let page = (y / 8) as usize;
        let idx = (x as usize) + (page * 128);
        let bit = (y % 8) as u8;
        buffer[idx] |= 1 << bit;
    }
}

fn draw_pacman(buffer: &mut [u8], cx: i32, cy: i32, radius: i32, mouth_open: bool) {
    let r2 = radius * radius;

    // Iterate bounding box
    for y in (cy - radius)..=(cy + radius) {
        for x in (cx - radius)..=(cx + radius) {
            let dist_sq = (x - cx).pow(2) + (y - cy).pow(2);

            if dist_sq <= r2 {
                // Mouth Logic (Wedge pointing right)
                if mouth_open {
                    if x > cx {
                        if (y - cy).abs() < (x - cx) {
                            continue; // Skip pixel (mouth gap)
                        }
                    }
                }
                set_pixel(buffer, x, y);
            }
        }
    }
}

fn draw_dots(buffer: &mut [u8], pacman_x: i32) {
    // Dots at x = 10, 25, 40, ... 115
    for x in (10..120).step_by(15) {
        // Only draw if Pacman hasn't eaten it (x > pacman_x + offset)
        if x > pacman_x + 5 {
            set_pixel(buffer, x, 16);
            set_pixel(buffer, x + 1, 16);
            set_pixel(buffer, x, 17);
            set_pixel(buffer, x + 1, 17);
        }
    }
}

// --- HARDWARE HELPERS ---

fn update_screen(spi: &SpiDevice, dc: &DigitalOutPin, buffer: &[u8]) {
    // Set Columns 0-127
    send_cmd(spi, dc, 0x21);
    send_cmd(spi, dc, 0);
    send_cmd(spi, dc, 127);

    // Set Pages 0-3
    send_cmd(spi, dc, 0x22);
    send_cmd(spi, dc, 0);
    send_cmd(spi, dc, 3);

    // Send Buffer
    dc.set_state(PinState::Active).unwrap(); // Data (High)
    spi.write(buffer).unwrap();
}

fn send_cmd(spi: &SpiDevice, dc: &DigitalOutPin, c: u8) {
    dc.set_state(PinState::Inactive).unwrap(); // Cmd (Low)
    spi.write(&[c]).unwrap();
}

export!(MyGuest);
