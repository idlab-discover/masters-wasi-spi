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
    frequency: u32,
    mode: u8,
}

#[derive(Deserialize)]
struct GpioConfig {
    pin: u8,
    initial: String,
}

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    fs::write(out_dir.join("memory.x"), include_bytes!("memory.x")).unwrap();
    println!("cargo:rustc-link-search={}", out_dir.display());
    println!("cargo:rerun-if-changed=memory.x");
    println!("cargo:rustc-link-arg-bins=--nmagic");
    println!("cargo:rustc-link-arg-bins=-Tlink.x");
    println!("cargo:rustc-link-arg-bins=-Tdefmt.x");

    let policy_str = fs::read_to_string("policy.toml").expect("CRITICAL: policy.toml is missing!");
    let policy: Policy =
        toml::from_str(&policy_str).expect("CRITICAL: Failed to parse policy.toml");

    let mut spi_initializations = quote! {
        let mut spi_hardware: alloc::vec::Vec<(alloc::string::String, alloc::boxed::Box<dyn spi::ErasedSpiDevice + Send>)> = alloc::vec::Vec::new();
    };

    // SPI
    for (name, config) in policy.spi.into_iter().flatten() {
        let sck_pin = format_ident!("PIN_{}", config.sck);
        let mosi_pin = format_ident!("PIN_{}", config.mosi);
        let miso_pin = format_ident!("PIN_{}", config.miso);
        let cs_pin = format_ident!("PIN_{}", config.cs);

        let block_num: u8 = config.block.trim_start_matches("SPI").parse().unwrap();
        let block_ident = format_ident!("{}", config.block);
        let spi_ident = format_ident!("spi{}", block_num);
        let cs_ident = format_ident!("cs{}", block_num);

        let freq = config.frequency;
        let (pol, pha) = match config.mode {
            0 => (quote!(embassy_rp::spi::Polarity::IdleLow), quote!(embassy_rp::spi::Phase::CaptureOnFirstTransition)),
            1 => (quote!(embassy_rp::spi::Polarity::IdleLow), quote!(embassy_rp::spi::Phase::CaptureOnSecondTransition)),
            2 => (quote!(embassy_rp::spi::Polarity::IdleHigh), quote!(embassy_rp::spi::Phase::CaptureOnFirstTransition)),
            3 => (quote!(embassy_rp::spi::Polarity::IdleHigh), quote!(embassy_rp::spi::Phase::CaptureOnSecondTransition)),
            _ => panic!("CRITICAL: SPI mode must be 0, 1, 2, or 3"),
        };

        spi_initializations.extend(quote! {
            let mut rp_config = embassy_rp::spi::Config::default();
            rp_config.frequency = #freq;
            rp_config.polarity = #pol;
            rp_config.phase = #pha;

            let #spi_ident = embassy_rp::spi::Spi::new_blocking(
                $p.#block_ident, $p.#sck_pin, $p.#mosi_pin, $p.#miso_pin, rp_config
            );
            let #cs_ident = embassy_rp::gpio::Output::new($p.#cs_pin, embassy_rp::gpio::Level::High);
            
            let device = embedded_hal_bus::spi::ExclusiveDevice::new_no_delay(#spi_ident, #cs_ident).unwrap();
            spi_hardware.push((alloc::string::String::from(#name), alloc::boxed::Box::new(device)));
        });
    }

    // GPIO
    let mut gpio_inserts = quote! { 
        let mut gpio_map: alloc::collections::BTreeMap<alloc::string::String, alloc::boxed::Box<dyn gpio::ErasedOutputPin + Send>> = alloc::collections::BTreeMap::new(); 
    };
    
    for (name, config) in policy.gpio.into_iter().flatten() {
        let pin_ident = format_ident!("PIN_{}", config.pin);
        let level = match config.initial.as_str() {
            "High" => quote! { embassy_rp::gpio::Level::High },
            "Low" => quote! { embassy_rp::gpio::Level::Low },
            _ => panic!("Invalid GPIO level"),
        };
        gpio_inserts.extend(quote! { 
            gpio_map.insert(
                alloc::string::String::from(#name), 
                alloc::boxed::Box::new(embassy_rp::gpio::Output::new($p.#pin_ident, #level))
            ); 
        });
    }

    let final_code = quote! {
        macro_rules! configure_hardware {
            ($p:expr) => {{
                #spi_initializations
                #gpio_inserts
                (spi_hardware, gpio_map) // Cleanly return just the two hardware collections
            }}
        }
    };

    fs::write(out_dir.join("hardware_policy.rs"), final_code.to_string()).unwrap();
}
