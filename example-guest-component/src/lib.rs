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
// 128x32 Configuration Commands from Python Script
const INIT_SEQUENCE: &[u8] = &[
    0xAE, // Display OFF
    0xD5, 0x80, // Clock div
    0xA8, 0x1F, // Multiplex: 32 rows (0x1F)
    0xD3, 0x00, // Offset: 0
    0x40, // Start Line: 0
    0x8D, 0x14, // Charge Pump ON
    0x20, 0x00, // Memory Mode: Horizontal
    0xA1, // Segment Remap (Flip X)
    0xC8, // COM Scan (Flip Y)
    0xDA, 0x02, // COM Pins: Sequential
    0x81, 0x8F, // Contrast
    0xD9, 0xF1, // Precharge
    0xDB, 0x40, // VCOM Detect
    0xA4, // Resume RAM
    0xA6, // Normal Display
    0xAF, // Display ON
];

struct MyGuest;

impl Guest for MyGuest {
    fn run() {
        println!("--- [Guest] Starting OLED Animation ---");

        // 1. SPI Setup
        let device_names = get_device_names();
        if device_names.is_empty() {
            println!("[Guest] Error: No SPI devices found.");
            return;
        }
        // Assuming the first device is correct (e.g. "display")
        let spi = open_device(&device_names[0]).expect("Failed to open SPI");

        // 2. GPIO Setup
        // DC: Active High (Low=Cmd, High=Data)
        let flags_dc = &[DigitalFlag::OUTPUT, DigitalFlag::ACTIVE_HIGH];

        // RES, VBATC, VDDC: Active Low
        // In Python these are driven LOW to activate/reset.
        // Using ACTIVE_LOW means: PinState::Active -> 0V (Low), PinState::Inactive -> 3.3V (High)
        let flags_active_low = &[DigitalFlag::OUTPUT, DigitalFlag::ACTIVE_LOW];

        let dc = DigitalOutPin::get("DC", flags_dc).expect("Err DC");
        let res = DigitalOutPin::get("RES", flags_active_low).expect("Err RES");
        let vbatc = DigitalOutPin::get("VBATC", flags_active_low).expect("Err VBATC");
        let vddc = DigitalOutPin::get("VDDC", flags_active_low).expect("Err VDDC");

        // 3. Power Cycle Sequence (Matches Python init_display)
        println!("[Guest] Power Cycling...");

        // Python: GPIO.output(PIN_VBATC, GPIO.HIGH) -> OFF
        // ActiveLow Inactive = High
        vbatc.set_state(PinState::Inactive).unwrap();
        vddc.set_state(PinState::Inactive).unwrap();
        delay_ms(100); // sleep(0.1)

        // Python: GPIO.output(PIN_VDDC, GPIO.LOW) -> Logic ON
        // ActiveLow Active = Low
        vddc.set_state(PinState::Active).unwrap();
        delay_ms(100); // sleep(0.1)

        // Python: GPIO.output(PIN_VBATC, GPIO.LOW) -> VBAT ON
        vbatc.set_state(PinState::Active).unwrap();
        delay_ms(100); // sleep(0.1)

        // 4. Reset Pulse
        // Python: High (Idle) -> Low (Reset) -> High (Idle)
        res.set_state(PinState::Inactive).unwrap(); // High
        delay_ms(1); // sleep(0.001)
        res.set_state(PinState::Active).unwrap(); // Low (Reset)
        delay_ms(10); // sleep(0.01)
        res.set_state(PinState::Inactive).unwrap(); // High

        // 5. Config Commands
        println!("[Guest] Sending Config...");
        for &byte in INIT_SEQUENCE {
            send_cmd(&spi, &dc, byte);
        }

        // 6. Animation Loop
        println!("[Guest] Running Animation Loop...");

        // 128 cols * 4 pages = 512 bytes
        let mut buffer = vec![0u8; 512];

        loop {
            // Move bar from Left (0) to Right (127)
            for x in 0..128 {
                // --- set_bar(x) logic ---
                // Clear buffer
                buffer.fill(0);

                // Draw vertical bar at column x across all 4 pages
                if x < 128 {
                    buffer[x] = 0xFF; // Page 0
                    buffer[x + 128] = 0xFF; // Page 1
                    buffer[x + 256] = 0xFF; // Page 2
                    buffer[x + 384] = 0xFF; // Page 3
                }

                // --- update_screen() logic ---
                // Set Column Address (0-127)
                send_cmd(&spi, &dc, 0x21);
                send_cmd(&spi, &dc, 0);
                send_cmd(&spi, &dc, 127);

                // Set Page Address (0-3)
                send_cmd(&spi, &dc, 0x22);
                send_cmd(&spi, &dc, 0);
                send_cmd(&spi, &dc, 3);

                // Send Data
                send_data(&spi, &dc, &buffer);

                // No sleep, run as fast as possible (like Python)
                // If it's too fast, uncomment the line below:
                // delay_ms(5);
            }
        }
    }
}

// Helper: Command (DC Low)
fn send_cmd(spi: &SpiDevice, dc: &DigitalOutPin, c: u8) {
    dc.set_state(PinState::Inactive).unwrap(); // Inactive = Low (because DC is ACTIVE_HIGH)
    spi.write(&[c]).unwrap();
}

// Helper: Data (DC High)
fn send_data(spi: &SpiDevice, dc: &DigitalOutPin, d: &[u8]) {
    dc.set_state(PinState::Active).unwrap(); // Active = High
    spi.write(d).unwrap();
}

export!(MyGuest);
