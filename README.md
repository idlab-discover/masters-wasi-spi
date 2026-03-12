<<<<<<< HEAD
# WASI SPI

This repository contains a full end-to-end implementation of SPI (Serial Peripheral Interface) support for Wasmtime. It provides a standardized **WIT interface** for SPI and enables Wasm components to communicate with physical hardware on a Linux host (like a Raspberry Pi) using the Component Model.

## Project Structure

This project is split into three parts that work together:

* **[`wasi-spi`](wasi-spi)**: A Rust library that implements the host-side logic for the SPI WIT interface. It exposes a trait so that any Wasmtime host can easily add SPI support.
* **[`host`](host)**: A generic Wasmtime runtime application. It uses the `wasi-spi` and `wasi-gpio` libraries to run Wasm components with access to the real hardware. It uses a policy file to map physical pins to virtual names.
* **[`guests`](guests/oled-screen)**: Example WebAssembly components. This includes a **Driver** component (which speaks raw SPI to a display peripheral) and **Application** components (like Pacman and Ball Screensaver) that rely on the driver to draw graphics.

---

## Design Decisions: The SPI WIT Interface

The most important part of this project is the `spi.wit` interface definition. There were two major architectural decisions involved in its design regarding how much control is given to the guest.

### 1. Configuration (Host vs. Guest)

Different sensors and screens require specific clock frequencies, bit orders, and clock modes (polarity and phase).

The main question is whether the **Host** or the **Guest** should be responsible for setting these parameters.

In this implementation, **the Guest configures the device**.

The alternative would be for the Host to set the configuration (e.g., via a config file) and present a ready-to-use device to the guest. The downside of that approach is that the Host becomes tightly coupled to the specific hardware attached to it. If you swap a sensor, you have to reconfigure and restart the Host.

By letting the Guest set the configuration (via the `configure` function in the WIT interface), the Host remains generic. It simply passes the raw SPI connection to the guest, and the guest decides how to talk to it.

While this may seem like the Guest implementations become more complex, the **Component Model** solves the complexity. We can wrap those specific settings inside a dedicated **Driver Component** (like `pmod-oled-driver`). The actual application component (like `pacman`) then links to that driver and uses high-level functions (like `set-pixel`), completely ignoring the low-level SPI configuration.

### 2. Device Discovery (Static vs. Dynamic)

The second decision is how a guest obtains a handle to a device: should the guest be able to open any device it wants, or only a predefined set allowed by the host?

The WIT interface uses `get-device-names` and `open-device` to support **both** workflows, leaving the final decision up to the host implementation.

* **Predefined Peripherals (Secure):** In a secure environment, you don't want a guest probing every hardware bus. The host implementation can read a policy file and only return a specific list of virtual names (e.g., `"screen"`, `"sensor"`) from `get-device-names`. The guest can only open what is explicitly allowed.
* **Dynamic Access (Open):** In a development environment or a system utility, you might want the guest to see everything. The host implementation could scan the `/dev/` directory and return every available SPI device.

This flexibility allows the same interface to be used for strict, sandboxed applications as well as general-purpose hardware tools.
=======
# run wasmtime in pico 2: blinky

run build.sh


## pico 2 pinout

BME280 Pin,Function,Pico 2 Pin
+,3.3V Power,3V3 (Pin 36)
-,Ground,GND (Pin 38)
C,SCK (Clock),GP18 (Pin 24)
D,MOSI (Data In),GP19 (Pin 25)
SDO,MISO (Data Out),GP16 (Pin 21)
CS,CS (Chip Select),GP17 (Pin 22)

PmodOLED Pin,Function,Pico 2 Pin Recommendation,Pico 2 Hardware Peripheral
1,CS (Chip Select),GP13 (Pin 17),SPI1 CSn
2,SDIN (MOSI),GP11 (Pin 15),SPI1 TX
3,Unused,Not Connected,-
4,SCLK (Clock),GP10 (Pin 14),SPI1 SCK
"5, 11",GND,GND (Pin 18 or any GND),Ground
"6, 12",VCC,3V3 OUT (Pin 36),3.3V Power
7,D/C (Data/Cmd),GP2 (Pin 4),Standard GPIO
8,RES (Reset),GP3 (Pin 5),Standard GPIO
9,VBATC (Bat Volt),GP4 (Pin 6),Standard GPIO
10,VDDC (Logic Volt),GP5 (Pin 7),Standard GPIO

## Raspberry pi 4 pinout

```
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
>>>>>>> pico/main
