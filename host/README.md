# Wasmtime Hardware Host

This is a generic **Wasmtime host application** designed to run WebAssembly components that require access to physical hardware.

It serves as the runtime environment that plugs in the host-side implementations for hardware capabilities:
* **SPI:** via the `wasi-spi` library.
* **GPIO:** via the external `wasi-gpio` library.

## Usage

The host takes a compiled WebAssembly component (`.wasm`) and executes it. It requires a **Policy File** to securely map the physical hardware pins to the virtual names requested by the guest.

```bash
cargo run -p host -- <COMPONENT_PATH> --policy-file <POLICY_PATH>
```

### Arguments

| Argument | Description |
| :--- | :--- |
| `<COMPONENT_PATH>` | The path to the compiled Wasm component to run (e.g., `pacman.wasm`). |
| `--policy-file` | Path to the `.toml` configuration file defining SPI and GPIO mappings. |

### Example

```bash
cargo run -p host -- \
  "./target/wasm32-wasip2/release/my_app.wasm" \
  --policy-file "./policies.toml"
```

## Example Policy

For a complete example of a policy configuration file, see the [oled-screen policy file](../guests/oled-screen/pmod-oled-driver/policies.toml) located in the `guests/oled-screen` folder.
