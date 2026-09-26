//! Reboot the central (right) half into BOOTSEL with Via's BootloaderJump.
//! The peripheral has no USB interface; use its bootmagic key instead.

use hidapi::HidApi;

// vendor_id and product_id from keyboard.toml.
const VID: u16 = 0x4c4b;
const PID: u16 = 0x4643;
const VIA_USAGE_PAGE: u16 = 0xff60;
const VIA_USAGE: u16 = 0x61;
const BOOTLOADER_JUMP: u8 = 0x0b;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api = HidApi::new()?;
    let info = api
        .device_list()
        .find(|d| {
            d.vendor_id() == VID && d.product_id() == PID && d.usage_page() == VIA_USAGE_PAGE && d.usage() == VIA_USAGE
        })
        .ok_or("liris60 not connected over USB")?;
    // Report ID 0, then the 32-byte Via report.
    let mut report = [0u8; 33];
    report[1] = BOOTLOADER_JUMP;
    info.open_device(&api)?.write(&report)?;
    println!("rebooting into BOOTSEL");
    Ok(())
}
