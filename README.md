# Wasmtime in Pico 2: Blinky & SPI Peripherals


This project demonstrates running WebAssembly on both a microcontroller (Raspberry Pi Pico 2) and a single-board computer (Raspberry Pi 4). 

It uses a WebAssembly guest to interface with a BME280 temperature sensor and display the readings on a PmodOLED screen. 
While the primary focus of this repository is demonstrating an SPI implementation, it also includes support for simple GPIO, delay, and logging functionalities.

## Repository Structure

- **`wit/spi.wit`**: Defines the SPI WebAssembly Interface Type (WIT) used by the guest to securely interface with the hardware.
- **Guest**: The hardware-agnostic application that reads the sensor and outputs to the display.
- **Hosts**: Two agnostic hosts are provided. One for the Pico 2 (host/) and one for the Raspberry Pi 4 (linux-host/).
- **`policy.toml`**: Present in both host directories. It tells the host which physical hardware pins and buses correspond to the labels requested by the guest.

## Execution Flow

The project follows a write-once, run-anywhere approach:

1. **Guest Compilation**: The guest code is compiled targeting `wasm32-unknown-unknown`.
2. **Componentization**: The compiled Wasm is transformed into a standard WebAssembly component using `wasm-tools`.
3. **Running on Raspberry Pi 4**: The resulting component can be executed directly by the Raspberry Pi 4 host, guided by its local `policy.toml`.
4. **Running on Pico 2**: Because of strict embedded constraints, a compiler script transforms the component into Pulley bytecode. This bytecode, alongside the Pico's `policy.toml`, is then baked directly into the Pico firmware.

## Build / Make

To execute the complete pipeline (compiling the guest, componentizing it, generating the Pulley bytecode, and building the firmwares), run the included build script:

```bash
./build.sh pico # or ./build.sh linux
```

note: I only tested the pico with a debug probe attached

## Hardware Pinouts

### Pico 2 Pinout

```text
Raspberry Pi Pico 2 Header
                           (Top View)
                              USB
                    GP0  [ 1] o o [40] VBUS
                    GP1  [ 2] o o [39] VSYS
                 Ground  [ 3] o o [38] Ground        (Sensor -)
  (OLED D/C)        GP2  [ 4] o o [37] 3V3_EN
  (OLED RES)        GP3  [ 5] o o [36] 3V3 OUT       (Sensor +, OLED VCC)
(OLED VBATC)        GP4  [ 6] o o [35] ADC_VREF
 (OLED VDDC)        GP5  [ 7] o o [34] GP28 / ADC2
                 Ground  [ 8] o o [33] Ground
                    GP6  [ 9] o o [32] GP27 / ADC1
                    GP7  [10] o o [31] GP26 / ADC0
                    GP8  [11] o o [30] RUN
                    GP9  [12] o o [29] GP22
                 Ground  [13] o o [28] Ground
 (OLED SCLK)       GP10  [14] o o [27] GP21
 (OLED SDIN)       GP11  [15] o o [26] GP20
                   GP12  [16] o o [25] GP19          (Sensor D / MOSI)
   (OLED CS)       GP13  [17] o o [24] GP18          (Sensor C / SCK)
  (OLED GND)     Ground  [18] o o [23] Ground
                   GP14  [19] o o [22] GP17          (Sensor CS)
                   GP15  [20] o o [21] GP16          (Sensor SDO / MISO)
```

### Raspberry Pi 4 Pinout

```text
Raspberry Pi 4 GPIO Header
                                   (Top View)

   (Sensor +)      3.3V Power [ 1] o o [ 2] 5V Power
               SDA / GPIO 2   [ 3] o o [ 4] 5V Power
               SCL / GPIO 3   [ 5] o o [ 6] Ground        (Sensor -)
                     GPIO 4   [ 7] o o [ 8] GPIO 14 (TXD)
                     Ground   [ 9] o o [10] GPIO 15 (RXD)
    (OLED DC)       GPIO 17   [11] o o [12] GPIO 18
   (OLED RES)       GPIO 27   [13] o o [14] Ground        (OLED GND)
 (OLED VBATC)       GPIO 22   [15] o o [16] GPIO 23       (OLED VDDC)
   (OLED VCC)    3.3V Power   [17] o o [18] GPIO 24
(Shared MOSI) MOSI / GPIO 10  [19] o o [20] Ground
 (Sensor SDO) MISO / GPIO 9   [21] o o [22] GPIO 25
(Shared SCLK) SCLK / GPIO 11  [23] o o [24] GPIO 8 / CE0  (Sensor CS)
                     Ground   [25] o o [26] GPIO 7 / CE1  (OLED CS)
                 EEPROM SDA   [27] o o [28] EEPROM SCL
                     GPIO 5   [29] o o [30] Ground
                     GPIO 6   [31] o o [32] GPIO 12
                    GPIO 13   [33] o o [34] Ground
                    GPIO 19   [35] o o [36] GPIO 16
                    GPIO 26   [37] o o [38] GPIO 20
                     Ground   [39] o o [40] GPIO 21
```
