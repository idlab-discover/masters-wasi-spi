use wit_bindgen::generate;
use my::hardware::spi;

generate!({
    path: "wit",
    world: "app",
    with: { "my:hardware/spi": generate }
});

struct MyGuest;

impl Guest for MyGuest {

    fn run(device: spi::SpiDevice) {
        let data = vec![1, 2, 3, 4];
        device.write(&data).ok();
        let _ = device.read(4);
    }

}

export!(MyGuest);