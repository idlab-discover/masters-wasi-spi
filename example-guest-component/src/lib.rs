use wit_bindgen::generate;

generate!({
    path: "wit",
    world: "app",
    with: {
        "wasi:spi/spi": generate,
    }
});

use crate::wasi::spi::spi::{Operation, get_device_names, open_device};

struct MyGuest;

impl Guest for MyGuest {
    fn run() {
        println!("--- [Guest] SPI device discovery ---");

        let device_names = get_device_names();

        if device_names.is_empty() {
            println!("[Guest] No SPI devices found");
            return;
        }

        println!(
            "[Guest] Found {} device(s): {:?}",
            device_names.len(),
            device_names
        );

        for name in device_names {
            println!("\n[Guest] Attempting to open device: {}", name);

            let device = match open_device(&name) {
                Ok(d) => d,
                Err(e) => {
                    println!("[Guest] Failed to open '{}': {:?}", name, e);
                    continue;
                }
            };

            println!(
                "[Guest] Successfully opened '{}'. Starting operations...",
                name
            );

            let write_data = [0xCA, 0xFE];
            println!("[Guest] write: {:02X?}", write_data);
            if let Err(e) = device.write(&write_data) {
                println!("[Guest] write failed: {:?}", e);
            }

            println!("[Guest] read 4 bytes");
            match device.read(4) {
                Ok(data) => println!("[Guest] read: {:02X?}", data),
                Err(e) => println!("[Guest] read failed: {:?}", e),
            }

            let tx = vec![0x9F, 0x00, 0x00, 0x00];
            println!("[Guest] transfer tx: {:02X?}", tx);
            match device.transfer(&tx) {
                Ok(rx) => println!("[Guest] transfer rx: {:02X?}", rx),
                Err(e) => println!("[Guest] transfer failed: {:?}", e),
            }

            println!("[Guest] transaction");
            let ops = vec![Operation::Write(vec![0x75]), Operation::Read(1)];

            match device.transaction(&ops) {
                Ok(results) => println!("[Guest] transaction result: {:?}", results),
                Err(e) => println!("[Guest] transaction failed: {:?}", e),
            }
        }

        println!("\n--- [Guest] Done ---");
    }
}

export!(MyGuest);
