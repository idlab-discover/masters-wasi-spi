{
  description = "Pico 2 W Rust Environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ rust-overlay.overlays.default ];
        pkgs = import nixpkgs {
          inherit system overlays;
          config.allowUnfree = true;
        };

        # Pico 2 uses Arm Cortex-M33 (thumbv8m)
        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          targets = [
            "thumbv8m.main-none-eabihf"
            "wasm32-unknown-unknown"
            "wasm32-wasip1"
            "wasm32-wasip2"
          ];
          extensions = [ "rust-src" "clippy" "llvm-tools-preview" ];
        };

      in {
        devShells.default = pkgs.mkShell {
          name = "pico2-shell";

          packages = [
            rustToolchain
            pkgs.elf2uf2-rs # Flashing tool
            pkgs.picotool # Official Pico utility (optional but good)
            pkgs.flip-link # Linker optimizer (standard in embedded rust)
            pkgs.cargo-generate
            pkgs.cargo-component
            pkgs.wasmtime
            pkgs.wac-cli
            pkgs.wasm-tools
            pkgs.wabt
            pkgs.probe-rs-tools
          ];
        };
      });
}
