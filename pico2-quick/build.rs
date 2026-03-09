//! This build script copies the `memory.x` file from the crate root into
//! a directory where the linker can always find it at build time.
//! For many projects this is optional, as the linker always searches the
//! project root directory -- wherever `Cargo.toml` is. However, if you
//! are using a workspace or have a more complicated build setup, this
//! build script becomes required. Additionally, by requesting that
//! Cargo re-run the build script whenever `memory.x` is changed,
//! updating `memory.x` ensures a rebuild of the application with the
//! new memory settings.

use quote::{format_ident, quote};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::PathBuf;

#[derive(Deserialize)]
struct Policy {
    spi: Option<BTreeMap<String, SpiConfig>>,
    gpio: Option<BTreeMap<String, GpioConfig>>,
}

#[derive(Deserialize)]
struct SpiConfig {
    block: String,
    sck: u8,
    mosi: u8,
    miso: u8,
    cs: u8,
}

#[derive(Deserialize)]
struct GpioConfig {
    pin: u8,
    initial: String,
}

fn main() {
    // Put `memory.x` in our output directory and ensure it's
    // on the linker search path.
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    fs::write(out_dir.join("memory.x"), include_bytes!("memory.x")).unwrap();
    println!("cargo:rustc-link-search={}", out_dir.display());

    // By default, Cargo will re-run a build script whenever
    // any file in the project changes. By specifying `memory.x`
    // here, we ensure the build script is only re-run when
    // `memory.x` is changed.
    println!("cargo:rerun-if-changed=memory.x");

    println!("cargo:rustc-link-arg-bins=--nmagic");
    println!("cargo:rustc-link-arg-bins=-Tlink.x");
    println!("cargo:rustc-link-arg-bins=-Tdefmt.x");

    // ==================================================================

    // 1. STRICTLY read and parse the policy. No fallbacks.
    let policy_str = fs::read_to_string("policy.toml").expect("CRITICAL: policy.toml is missing!");
    let policy: Policy =
        toml::from_str(&policy_str).expect("CRITICAL: Failed to parse policy.toml");

    let mut spi_initializations = quote! {
        let mut spi0_opt = None;
        let mut spi1_opt = None;
        let mut device_map = alloc::collections::BTreeMap::new();
    };

    // 1. .into_iter().flatten() handles both Some and None effortlessly!
    for (name, config) in policy.spi.into_iter().flatten() {
        let sck_pin = format_ident!("PIN_{}", config.sck);
        let mosi_pin = format_ident!("PIN_{}", config.mosi);
        let miso_pin = format_ident!("PIN_{}", config.miso);
        let cs_pin = format_ident!("PIN_{}", config.cs);

        // 2. Dynamically extract the block number (e.g., "SPI0" -> 0u8)
        let block_num: u8 = config
            .block
            .trim_start_matches("SPI")
            .parse()
            .expect("CRITICAL: SPI block must be like 'SPI0' or 'SPI1'");

        // 3. Generate the exact identifiers
        let block_ident = format_ident!("{}", config.block); // e.g., SPI0
        let spi_ident = format_ident!("spi{}", block_num); // e.g., spi0
        let cs_ident = format_ident!("cs{}", block_num); // e.g., cs0
        let opt_ident = format_ident!("spi{}_opt", block_num); // e.g., spi0_opt

        spi_initializations.extend(quote! {
            let #spi_ident = embassy_rp::spi::Spi::new_blocking(
                $p.#block_ident, $p.#sck_pin, $p.#mosi_pin, $p.#miso_pin, embassy_rp::spi::Config::default()
            );
            let #cs_ident = embassy_rp::gpio::Output::new($p.#cs_pin, embassy_rp::gpio::Level::High);
            #opt_ident = Some((#spi_ident, #cs_ident));
            device_map.insert(alloc::string::String::from(#name), #block_num);
        });
    }

    let mut gpio_inserts = quote! { let mut gpio_map = alloc::collections::BTreeMap::new(); };

    // Flattened GPIO loop
    for (name, config) in policy.gpio.into_iter().flatten() {
        let pin_ident = format_ident!("PIN_{}", config.pin);
        let level = match config.initial.as_str() {
            "High" => quote! { embassy_rp::gpio::Level::High },
            "Low" => quote! { embassy_rp::gpio::Level::Low },
            _ => panic!("CRITICAL: Invalid GPIO initial level in policy!"),
        };

        gpio_inserts.extend(quote! {
            gpio_map.insert(
                alloc::string::String::from(#name),
                embassy_rp::gpio::Output::new($p.#pin_ident, #level)
            );
        });
    }

    let final_code = quote! {
        macro_rules! configure_hardware {
            ($p:expr) => {{
                #spi_initializations
                #gpio_inserts
                (device_map, spi0_opt, spi1_opt, gpio_map)
            }}
        }
    };

    fs::write(out_dir.join("hardware_policy.rs"), final_code.to_string()).unwrap();
}
