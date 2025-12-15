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
use crate::wasi::spi::spi::{Config, Mode, SpiDevice, get_device_names, open_device};

// --- CONSTANTS ---
const INIT_SEQUENCE: &[u8] = &[
    0xAE, // Display OFF
    0xD5, 0x80, // Clock div
    0xA8, 0x1F, // Multiplex: 32 rows
    0xD3, 0x00, // Offset: 0
    0x40, // Start Line: 0
    0x8D, 0x14, // Charge Pump ON
    // Using Page Addressing (0x02) to prevent screen tearing/shifting
    0x20, 0x02, 0xA1, // Segment Remap (Flip X)
    0xC8, // COM Scan (Flip Y)
    0xDA, 0x02, // COM Pins: Sequential
    0x81, 0x8F, // Contrast
    0xD9, 0xF1, // Precharge
    0xDB, 0x40, // VCOM Detect
    0xA4, // Resume RAM
    0xA6, // Normal Display
    0xAF, // Display ON
];

const WIDTH: i32 = 128;
const HEIGHT: i32 = 32;

struct MyGuest;

impl Guest for MyGuest {
    fn run() {
        println!("--- [Guest] Starting Pacman (8MHz) ---");

        // 1. Setup SPI
        let device_names = get_device_names();
        if device_names.is_empty() {
            return;
        }
        let spi = open_device(&device_names[0]).expect("No SPI");

        // --- SPI CONFIGURATION ---
        let config = Config {
            frequency: 8_000_000,
            mode: Mode::Mode0,
            lsb_first: false,
        };
        // FIX: Pass 'config' by value, not reference
        spi.configure(config).expect("Failed to configure SPI");

        // 2. Setup GPIO
        // DC is Active High (Low=Cmd, High=Data)
        let flags_dc = &[DigitalFlag::OUTPUT, DigitalFlag::ACTIVE_HIGH];
        // RES, VBATC, VDDC are Active Low (Low = ON)
        let flags_pwr = &[DigitalFlag::OUTPUT, DigitalFlag::ACTIVE_LOW];

        let dc = DigitalOutPin::get("DC", flags_dc).expect("Err DC");
        let res = DigitalOutPin::get("RES", flags_pwr).expect("Err RES");
        let vbatc = DigitalOutPin::get("VBATC", flags_pwr).expect("Err VBATC");
        let vddc = DigitalOutPin::get("VDDC", flags_pwr).expect("Err VDDC");

        // 3. Power Cycle
        // Start OFF (Inactive = High)
        vbatc.set_state(PinState::Inactive).unwrap();
        vddc.set_state(PinState::Inactive).unwrap();
        delay_ms(100);

        // Turn ON (Active = Low)
        vddc.set_state(PinState::Active).unwrap(); // Logic ON
        delay_ms(100);
        vbatc.set_state(PinState::Active).unwrap(); // VBAT ON
        delay_ms(100);

        // 4. Reset Pulse
        res.set_state(PinState::Inactive).unwrap();
        delay_ms(1);
        res.set_state(PinState::Active).unwrap(); // Reset
        delay_ms(10);
        res.set_state(PinState::Inactive).unwrap(); // Operational

        // 5. Config
        for &c in INIT_SEQUENCE {
            send_cmd(&spi, &dc, c);
        }

        // 6. Animation Loop
        let mut buffer = vec![0u8; 512];
        let mut pos_x: i32 = -15; // Start off-screen
        let mut mouth_open = true;
        let mut frame_count = 0;

        println!("[Guest] Running Pacman Loop...");

        loop {
            // A. Clear Buffer
            buffer.fill(0);

            // B. Draw Food
            draw_dots(&mut buffer, pos_x);

            // C. Draw Pacman
            draw_pacman(&mut buffer, pos_x, 16, 10, mouth_open);

            // D. Update Screen (Robust Method)
            update_screen_robust(&spi, &dc, &buffer);

            // E. Animation Logic
            pos_x += 3;
            frame_count += 1;

            if frame_count % 2 == 0 {
                mouth_open = !mouth_open;
            }

            if pos_x > WIDTH + 15 {
                pos_x = -15;
            }

            delay_ms(10);
        }
    }
}

// --- PACMAN LOGIC ---

fn draw_pacman(buffer: &mut [u8], cx: i32, cy: i32, radius: i32, mouth_open: bool) {
    let r2 = radius * radius;

    for y in (cy - radius)..=(cy + radius) {
        for x in (cx - radius)..=(cx + radius) {
            let dist_sq = (x - cx).pow(2) + (y - cy).pow(2);

            if dist_sq <= r2 {
                if mouth_open {
                    if x > cx {
                        if (y - cy).abs() < (x - cx) {
                            continue;
                        }
                    }
                }
                set_pixel(buffer, x, y);
            }
        }
    }
}

fn draw_dots(buffer: &mut [u8], pacman_x: i32) {
    let mut x = 10;
    while x < 120 {
        if x > pacman_x + 5 {
            set_pixel(buffer, x, 16);
            set_pixel(buffer, x + 1, 16);
            set_pixel(buffer, x, 17);
            set_pixel(buffer, x + 1, 17);
        }
        x += 15;
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

// --- HARDWARE HELPERS ---

fn update_screen_robust(spi: &SpiDevice, dc: &DigitalOutPin, buffer: &[u8]) {
    for page in 0..4 {
        send_cmd(spi, dc, 0xB0 | page);
        send_cmd(spi, dc, 0x00);
        send_cmd(spi, dc, 0x10);

        let start = (page as usize) * 128;
        let end = start + 128;

        dc.set_state(PinState::Active).unwrap(); // Data
        spi.write(&buffer[start..end]).unwrap();
    }
}

fn send_cmd(spi: &SpiDevice, dc: &DigitalOutPin, c: u8) {
    dc.set_state(PinState::Inactive).unwrap(); // Cmd (Low)
    spi.write(&[c]).unwrap();
}

export!(MyGuest);
