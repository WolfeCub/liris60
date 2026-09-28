{
  description = "liris60 RMK firmware";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    rmkit = {
      url = "github:rmk-rs/rmkit/v0.1.0";
      flake = false;
    };
  };

  outputs = { nixpkgs, rust-overlay, rmkit, ... }:
    let
      systems = [ "aarch64-darwin" "x86_64-darwin" "aarch64-linux" "x86_64-linux" ];
      forAllSystems = f: nixpkgs.lib.genAttrs systems (system:
        f (import nixpkgs { inherit system; overlays = [ rust-overlay.overlays.default ]; }));
    in
    {
      devShells = forAllSystems (pkgs: {
        default = pkgs.mkShell {
          packages = [
            # Toolchain, target and components from rust-toolchain.toml.
            (pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml)
            # Linker set in build.rs.
            pkgs.flip-link
            # Flashes a board in BOOTSEL over USB.
            pkgs.picotool
            # Not in nixpkgs; built from the pinned rmkit input.
            (pkgs.rustPlatform.buildRustPackage {
              pname = "rmkit";
              version = "0.1.0";
              src = rmkit;
              cargoLock.lockFile = "${rmkit}/Cargo.lock";
              doCheck = false;
            })
          ] ++ pkgs.lib.optionals pkgs.stdenv.isLinux [
            # hidapi (tools/bootsel) links libudev on Linux.
            pkgs.pkg-config
            pkgs.udev
          ];
        };
      });
    };
}
