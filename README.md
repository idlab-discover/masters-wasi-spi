# WASI SPI

This repository contains a full end-to-end implementation of SPI (Serial Peripheral Interface) support for Wasmtime. It enables Wasm components to communicate with physical hardware on a Linux host (like a Raspberry Pi) using the Component Model.

## Project Structure

This project is split into three parts that work together:

* **[`wasi-spi`](wasi-spi)**: A Rust library that implements the host-side logic for the SPI WIT interface. It exposes a trait so that any Wasmtime host can easily add SPI support without needing to re-write the low-level Linux hardware code.
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

While this means the guest has to handle low-level details, the **Component Model** solves the complexity. We can wrap those specific settings inside a dedicated **Driver Component** (like `pmod-oled-driver`). The actual application component (like `pacman`) then links to that driver and uses high-level functions (like `set-pixel`), completely ignoring the low-level SPI configuration.

### 2. Device Discovery (Static vs. Dynamic)

The second decision is how a guest obtains a handle to a device: should the guest be able to open any device it wants, or only a predefined set allowed by the host?

The WIT interface uses `get-device-names` and `open-device` to support **both** workflows, leaving the final decision up to the host implementation.

* **Predefined Peripherals (Secure):** In a secure environment, you don't want a guest probing every hardware bus. The host implementation can read a policy file and only return a specific list of virtual names (e.g., `"screen"`, `"sensor"`) from `get-device-names`. The guest can only open what is explicitly allowed.
* **Dynamic Access (Open):** In a development environment or a system utility, you might want the guest to see everything. The host implementation could scan the `/dev/` directory and return every available SPI device.

This flexibility allows the same interface to be used for strict, sandboxed applications as well as general-purpose hardware tools.
