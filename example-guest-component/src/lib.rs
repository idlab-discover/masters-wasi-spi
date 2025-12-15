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

use crate::wasi::gpio::digital::{DigitalFlag, DigitalOutPin, PinState};
use crate::wasi::spi::spi::{SpiDevice, get_device_names, open_device};
// Import the standalone delay function directly
use crate::wasi::gpio::delay::delay_ms;

// Standard SSD1306 Commands
const CMD_DISPLAY_OFF: u8 = 0xAE;
const CMD_SET_CHARGE_PUMP: u8 = 0x8D;
const CMD_SET_SEGMENT_REMAP_127: u8 = 0xA1;
const CMD_SET_COM_SCAN_REVERSE: u8 = 0xC8;
const CMD_DISPLAY_ON: u8 = 0xAF;
const CMD_SET_PRECHARGE: u8 = 0xD9;
const CMD_SET_CONTRAST: u8 = 0x81;

struct MyGuest;

impl Guest for MyGuest {
    fn run() {
        println!("--- [Guest] Starting OLED Driver ---");

        // 1. Open SPI Device
        let device_names = get_device_names();
        if device_names.is_empty() {
            println!("[Guest] Error: No SPI devices found.");
            return;
        }
        let spi_name = &device_names[0];
        println!("[Guest] Opening SPI device: {}", spi_name);
        let spi = open_device(spi_name).expect("Failed to open SPI device");

        // 2. Open GPIO Pins
        // We MUST provide ACTIVE_HIGH (or low) to satisfy the host builder
        let output_flags = &[DigitalFlag::OUTPUT, DigitalFlag::ACTIVE_HIGH];

        println!("[Guest] Opening GPIO pins...");
        let dc_pin = DigitalOutPin::get("DC", output_flags).expect("Failed to get D/C pin");
        let res_pin = DigitalOutPin::get("RES", output_flags).expect("Failed to get RES pin");
        let vddc_pin = DigitalOutPin::get("VDDC", output_flags).expect("Failed to get VDDC pin");
        let vbatc_pin = DigitalOutPin::get("VBATC", output_flags).expect("Failed to get VBATC pin");

        // 3. Power Up Sequence (PmodOLED specific)
        println!("[Guest] Powering up...");

        // A. Turn on Logic Voltage (VDDC)
        vddc_pin.set_state(PinState::Active).unwrap();
        delay_ms(5); // Wait for logic to stabilize

        // B. Turn on Display Voltage (VBATC)
        vbatc_pin.set_state(PinState::Active).unwrap();
        delay_ms(100); // Wait for high voltage to stabilize

        // 4. Reset Sequence
        println!("[Guest] Resetting...");
        res_pin.set_state(PinState::Active).unwrap(); // High
        delay_ms(1);
        res_pin.set_state(PinState::Inactive).unwrap(); // Low (Reset)
        delay_ms(10); // Keep in reset
        res_pin.set_state(PinState::Active).unwrap(); // High (Release)
        delay_ms(10);

        // 5. Initialization Sequence
        println!("[Guest] Sending Init Commands...");
        let init_cmds = [
            CMD_DISPLAY_OFF,
            CMD_SET_CHARGE_PUMP,
            0x14, // Enable Charge Pump
            CMD_SET_PRECHARGE,
            0xF1, // Set Pre-Charge Period
            CMD_SET_CONTRAST,
            0xFF,                      // Max Contrast
            CMD_SET_SEGMENT_REMAP_127, // Invert X
            CMD_SET_COM_SCAN_REVERSE,  // Invert Y
            CMD_DISPLAY_ON,
        ];

        for cmd in init_cmds {
            send_command(&spi, &dc_pin, cmd);
        }

        // 6. Clear Screen
        println!("[Guest] Clearing Screen...");
        let clear_buffer = vec![0u8; 512];
        send_data(&spi, &dc_pin, &clear_buffer);

        // 7. Draw Pattern
        println!("[Guest] Drawing Pattern...");
        let mut draw_buffer = vec![0u8; 512];
        for i in 0..512 {
            // Draw a diagonal-ish striped pattern
            if i % 2 == 0 {
                draw_buffer[i] = 0xFF;
            } else {
                draw_buffer[i] = 0x00;
            }
        }
        send_data(&spi, &dc_pin, &draw_buffer);

        println!("--- [Guest] Drawing Complete ---");

        // Keep screen on for 5 seconds so you can see it before exit
        delay_ms(5000);

        // Optional: Turn off power before exiting
        vbatc_pin.set_state(PinState::Inactive).unwrap();
        vddc_pin.set_state(PinState::Inactive).unwrap();
    }
}

// Helper: Send Command (D/C = Low)
fn send_command(spi: &SpiDevice, dc_pin: &DigitalOutPin, cmd: u8) {
    dc_pin.set_state(PinState::Inactive).unwrap();
    spi.write(&[cmd]).expect("SPI write failed");
}

// Helper: Send Data (D/C = High)
fn send_data(spi: &SpiDevice, dc_pin: &DigitalOutPin, data: &[u8]) {
    dc_pin.set_state(PinState::Active).unwrap();
    spi.write(data).expect("SPI write failed");
}

export!(MyGuest);
