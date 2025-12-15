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

use crate::wasi::spi::spi::{SpiDevice, get_device_names, open_device};
// 1. Import DigitalOutPin instead of 'get'
use crate::wasi::gpio::digital::{DigitalFlag, DigitalOutPin, PinState};

// Standard SSD1306 Commands
const CMD_DISPLAY_OFF: u8 = 0xAE;
const CMD_SET_CHARGE_PUMP: u8 = 0x8D;
const CMD_SET_SEGMENT_REMAP_127: u8 = 0xA1;
const CMD_SET_COM_SCAN_REVERSE: u8 = 0xC8;
const CMD_DISPLAY_ON: u8 = 0xAF;

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
        println!("[Guest] Opening GPIO pins...");

        // 2. Use DigitalFlag::OUTPUT (Uppercase)
        let output_flags = &[DigitalFlag::OUTPUT];

        // 3. Use DigitalOutPin::get()
        let dc_pin = DigitalOutPin::get("DC", output_flags).expect("Failed to get D/C pin");
        let res_pin = DigitalOutPin::get("RES", output_flags).expect("Failed to get RES pin");

        if !dc_pin.is_ready() || !res_pin.is_ready() {
            println!("[Guest] Error: GPIO pins not ready.");
            return;
        }

        // 3. Hardware Reset Sequence
        println!("[Guest] Resetting OLED...");
        res_pin.set_state(PinState::Active).unwrap(); // High
        block_delay_ms(1);
        res_pin.set_state(PinState::Inactive).unwrap(); // Low (Reset)
        block_delay_ms(10);
        res_pin.set_state(PinState::Active).unwrap(); // High (Operational)
        block_delay_ms(10);

        // 4. Initialization Sequence
        println!("[Guest] Sending Initialization Commands...");
        let init_cmds = [
            CMD_DISPLAY_OFF,
            CMD_SET_CHARGE_PUMP,
            0x14,
            CMD_SET_SEGMENT_REMAP_127,
            CMD_SET_COM_SCAN_REVERSE,
            CMD_DISPLAY_ON,
        ];

        for cmd in init_cmds {
            send_command(&spi, &dc_pin, cmd);
        }

        // 5. Clear Screen
        println!("[Guest] Clearing Screen...");
        let clear_buffer = vec![0u8; 512];
        send_data(&spi, &dc_pin, &clear_buffer);

        // 6. Draw Pattern
        println!("[Guest] Drawing Pattern...");
        let mut draw_buffer = vec![0u8; 512];
        for i in 0..128 {
            if i < 512 {
                draw_buffer[i] = 1 << (i % 8);
            }
        }
        send_data(&spi, &dc_pin, &draw_buffer);

        println!("--- [Guest] Drawing Complete ---");
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

fn block_delay_ms(ms: u32) {
    // Fallback delay
    let iterations = ms * 10000;
    let mut _volatile = 0;
    for _ in 0..iterations {
        _volatile += 1;
    }
}

export!(MyGuest);
