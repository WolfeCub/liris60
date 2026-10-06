//! Send Via commands to the central (right) half.
//! The peripheral has no USB interface; use its bootmagic key instead.

use std::str::FromStr;

use hidapi::HidApi;
use rmk_types::protocol::vial::{VIAL_EP_SIZE, ViaCommand};

// vendor_id and product_id from keyboard.toml.
const VID: u16 = 0x4c4b;
const PID: u16 = 0x4643;
// Via's raw HID interface, only defined inline in rmk's report descriptor:
// https://docs.rs/crate/rmk/0.9.0/source/src/hid.rs#48
const VIA_USAGE_PAGE: u16 = 0xff60;
const VIA_USAGE: u16 = 0x61;

const USAGE: &str = "expected bootsel or reset";

enum Command {
    /// Reboot into BOOTSEL.
    Bootsel,
    /// Wipe storage; the board reboots and reloads the keymap from keyboard.toml.
    Reset,
}

impl FromStr for Command {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "bootsel" => Ok(Self::Bootsel),
            "reset" => Ok(Self::Reset),
            _ => Err(format!("unknown command {s:?}, {USAGE}")),
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let command: Command = std::env::args().nth(1).ok_or(USAGE)?.parse()?;
    let (via_command, message) = match command {
        Command::Bootsel => (ViaCommand::BootloaderJump, "rebooting into BOOTSEL"),
        Command::Reset => (ViaCommand::EepromReset, "wiping storage, the keymap reloads from keyboard.toml"),
    };

    let api = HidApi::new()?;
    let info = api
        .device_list()
        .find(|d| {
            d.vendor_id() == VID && d.product_id() == PID && d.usage_page() == VIA_USAGE_PAGE && d.usage() == VIA_USAGE
        })
        .ok_or("liris60 not connected over USB")?;
    // Report ID 0, then the Via report.
    let mut report = [0u8; 1 + VIAL_EP_SIZE];
    report[1] = via_command as u8;
    info.open_device(&api)?.write(&report)?;
    println!("{message}");
    Ok(())
}
