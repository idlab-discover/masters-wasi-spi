{
  description = "A declarative development environment for the wasi-spi thesis";

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

        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          targets = [ "wasm32-wasip1" "wasm32-wasip2" ];
          extensions = [ "rust-src" "clippy" ];
        };

      in {
        devShells.default = pkgs.mkShell {
          name = "wasi-thesis-shell";

          packages = [
            rustToolchain
            pkgs.wasmtime
            pkgs.cargo-component
            pkgs.jetbrains.rust-rover
            pkgs.lld
          ];
        };
      });
}
