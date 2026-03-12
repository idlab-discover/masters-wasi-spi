extern crate alloc;

use crate::my::debug::logging::log;
use crate::wasi::spi::spi::SpiDevice; // Removed open_device
use alloc::vec::Vec;

pub struct Bme280 {
    spi: SpiDevice,
    calib: Bme280Calib,
}

struct Bme280Calib {
    dig_t1: u16,
    dig_t2: i16,
    dig_t3: i16,
    dig_h1: u8,
    dig_h2: i16,
    dig_h3: u8,
    dig_h4: i16,
    dig_h5: i16,
    dig_h6: i8,
}

impl Bme280 {
    // We now take ownership of the SpiDevice directly
    pub fn new(spi: SpiDevice) -> Self {
        log("Initializing BME280...");

        let chip_id = read_register(&spi, 0xD0, 1)[0];
        if chip_id != 0x60 {
            log(&alloc::format!(
                "Warning: BME280 expected Chip ID 0x60, got 0x{:X}",
                chip_id
            ));
        }

        let calib = read_calibration_data(&spi);

        write_register(&spi, 0xF2, 0x01); // ctrl_hum (Humidity x1)
        write_register(&spi, 0xF4, 0x27); // ctrl_meas (Temp x1, Pressure x1, Normal)

        Self { spi, calib }
    }

    pub fn read(&self) -> (f32, f32) {
        let raw_t = read_raw_temp(&self.spi);
        let raw_h = read_raw_humidity(&self.spi);

        let (temp_c, t_fine) = compensate_temperature(raw_t, &self.calib);
        let humidity = compensate_humidity(raw_h, t_fine, &self.calib);

        (temp_c, humidity)
    }
}

// --- Internal Helper Functions ---

fn read_register(spi: &SpiDevice, reg: u8, len: u64) -> Vec<u8> {
    let mut tx_buf = alloc::vec![0u8; (len + 1) as usize];
    tx_buf[0] = reg | 0x80;
    let rx_buf = spi.transfer(&tx_buf).unwrap();
    rx_buf[1..].to_vec()
}

fn write_register(spi: &SpiDevice, reg: u8, value: u8) {
    let _ = spi.write(&[reg & !0x80, value]).unwrap();
}

fn read_calibration_data(spi: &SpiDevice) -> Bme280Calib {
    let t_data = read_register(spi, 0x88, 6);
    let dig_t1 = (t_data[0] as u16) | ((t_data[1] as u16) << 8);
    let dig_t2 = (t_data[2] as i16) | ((t_data[3] as i16) << 8);
    let dig_t3 = (t_data[4] as i16) | ((t_data[5] as i16) << 8);

    let dig_h1 = read_register(spi, 0xA1, 1)[0];

    let h_data = read_register(spi, 0xE1, 7);
    let dig_h2 = (h_data[0] as i16) | ((h_data[1] as i16) << 8);
    let dig_h3 = h_data[2];

    let dig_h4 = ((h_data[3] as i16) << 4) | ((h_data[4] as i16) & 0x0F);
    let dig_h5 = ((h_data[5] as i16) << 4) | ((h_data[4] as i16) >> 4);
    let dig_h6 = h_data[6] as i8;

    Bme280Calib {
        dig_t1,
        dig_t2,
        dig_t3,
        dig_h1,
        dig_h2,
        dig_h3,
        dig_h4,
        dig_h5,
        dig_h6,
    }
}

fn read_raw_temp(spi: &SpiDevice) -> i32 {
    let data = read_register(spi, 0xFA, 3);
    let msb = (data[0] as i32) << 12;
    let lsb = (data[1] as i32) << 4;
    let xlsb = (data[2] as i32) >> 4;
    msb | lsb | xlsb
}

fn read_raw_humidity(spi: &SpiDevice) -> i32 {
    let data = read_register(spi, 0xFD, 2);
    let msb = (data[0] as i32) << 8;
    let lsb = data[1] as i32;
    msb | lsb
}

fn compensate_temperature(adc_t: i32, calib: &Bme280Calib) -> (f32, f64) {
    let adc_t = adc_t as f64;
    let dig_t1 = calib.dig_t1 as f64;
    let dig_t2 = calib.dig_t2 as f64;
    let dig_t3 = calib.dig_t3 as f64;

    let var1 = (adc_t / 16384.0 - dig_t1 / 1024.0) * dig_t2;
    let var2 =
        ((adc_t / 131072.0 - dig_t1 / 8192.0) * (adc_t / 131072.0 - dig_t1 / 8192.0)) * dig_t3;

    let t_fine = var1 + var2;
    let temp = t_fine / 5120.0;
    (temp as f32, t_fine)
}

fn compensate_humidity(adc_h: i32, t_fine: f64, calib: &Bme280Calib) -> f32 {
    let adc_h = adc_h as f64;
    let mut var_h = t_fine - 76800.0;

    var_h = (adc_h - (calib.dig_h4 as f64 * 64.0 + calib.dig_h5 as f64 / 16384.0 * var_h))
        * (calib.dig_h2 as f64 / 65536.0
            * (1.0
                + calib.dig_h6 as f64 / 67108864.0
                    * var_h
                    * (1.0 + calib.dig_h3 as f64 / 67108864.0 * var_h)));
    var_h = var_h * (1.0 - calib.dig_h1 as f64 * var_h / 524288.0);

    if var_h > 100.0 {
        var_h = 100.0;
    } else if var_h < 0.0 {
        var_h = 0.0;
    }

    var_h as f32
}
