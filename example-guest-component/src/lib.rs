use wit_bindgen::generate;

generate!({
    path: "wit",
    world: "app",
    with: {
        "wasi:spi/spi": generate,
    }
});

use crate::wasi::spi::spi::{NamedDevice, Operation, get_devices};

struct MyGuest;

impl Guest for MyGuest {
    fn run() {
        println!("--- [Guest] SPI device discovery ---");

        let devices: Vec<NamedDevice> = get_devices();

        if devices.is_empty() {
            println!("[Guest] No SPI devices found");
            return;
        }

        println!("[Guest] Found {} device(s)", devices.len());

        for named in devices {
            let name = named.name;
            let device = named.device;

            println!("\n[Guest] Using device: {}", name);

            // WRITE
            let write_data = [0xCA, 0xFE];
            println!("[Guest] write: {:02X?}", write_data);
            if let Err(e) = device.write(&write_data) {
                println!("[Guest] write failed: {:?}", e);
                continue;
            }

            // READ
            println!("[Guest] read 4 bytes");
            match device.read(4) {
                Ok(data) => println!("[Guest] read: {:02X?}", data),
                Err(e) => {
                    println!("[Guest] read failed: {:?}", e);
                    continue;
                }
            }

            // TRANSFER (full-duplex)
            let tx = vec![0x9F, 0x00, 0x00, 0x00];
            println!("[Guest] transfer tx: {:02X?}", tx);

            match device.transfer(&tx) {
                Ok(rx) => println!("[Guest] transfer rx: {:02X?}", rx),
                Err(e) => {
                    println!("[Guest] transfer failed: {:?}", e);
                    continue;
                }
            }

            // TRANSACTION
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
