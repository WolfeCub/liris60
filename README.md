<h1 align="center">
  liris60
</h1>

<p align="center">
  <img width="2499" height="1182" alt="image" src="https://github.com/user-attachments/assets/1fb32279-c6e6-4154-87f6-d7805f7dd49b" />
</p>

A handwired split keyboard running [RMK](https://rmk.rs).

- Lily58/Iris hybrid that merges their thumb clusters, designed with [Cosmos](https://ryanis.cool/cosmos)
- Two Waveshare RP2040-Zeros, one per half
- The right half connects over USB and talks to the left half over UART through a TRRS cable
- Wiring, layers and the keymap are all in `keyboard.toml`

## Setup

- `nix develop` (or `direnv allow`) provides the Rust toolchain, `flip-link`, `picotool` and `rmkit`
- Builds need `rmkit`, which generates the Vial layout from `keyboard.toml`

## Flashing

- **Right half:**
  ```sh
  cargo bootsel
  cargo run --release --bin central
  ```
- **Left half:** hold the top-left key while plugging it in, then run
  ```sh
  cargo run --release --bin peripheral
  ```
- Keymap changes only need the right half reflashed
- `cargo run` uses `picotool` to flash whichever board is in BOOTSEL

## `cargo bootsel`

Reboots the right half into its bootloader (BOOTSEL) without touching the board.

- Sends Via's BootloaderJump command over USB (source in `tools/via`)
- Only works on the right half, since the left half has no USB interface of its own
- On macOS it may need Input Monitoring permission for your terminal
- To enter BOOTSEL without it, hold a half's top outer key while plugging it in, or press its BOOT button

## `cargo reset-layout`

Wipes any stored Vial changes. The keymap falls back to `keyboard.toml`.

- Sends Via's EepromReset command over USB, then the right half reboots
