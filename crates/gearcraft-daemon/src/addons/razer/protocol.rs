//! Razer HID protocol (OpenRazer / OpenRGB compatible).
//!
//! Wire format: 90-byte report (`0x5A`), CRC = XOR of bytes [2..88).

use gearcraft_core::RgbColor;

pub const REPORT_LEN: usize = 0x5A;

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum LedId {
    ScrollWheel = 0x01,
    Battery = 0x03,
    Logo = 0x04,
    Backlight = 0x05,
}

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum Effect {
    Off = 0x00,
    Static = 0x01,
    Breathing = 0x02,
    Spectrum = 0x04,
}

pub fn new_report() -> [u8; REPORT_LEN] {
    let mut r = [0u8; REPORT_LEN];
    r[0] = 0x00;
    r[1] = 0x1F;
    r
}

fn set_header(r: &mut [u8], data_size: u8, class: u8, cmd: u8) {
    r[2] = 0x00;
    r[3] = 0x00;
    r[4] = 0x00;
    r[5] = data_size;
    r[6] = class;
    r[7] = cmd;
}

fn finalize(r: &mut [u8; REPORT_LEN]) {
    let mut crc = 0u8;
    for b in r.iter().take(88).skip(2) {
        crc ^= *b;
    }
    r[88] = crc;
    r[89] = 0x00;
}

pub fn set_led_effect(led: LedId, effect: Effect) -> [u8; REPORT_LEN] {
    let mut r = new_report();
    set_header(&mut r, 0x03, 0x03, 0x02);
    r[8] = 0x01;
    r[9] = led as u8;
    r[10] = effect as u8;
    finalize(&mut r);
    r
}

pub fn set_led_rgb(led: LedId, color: &RgbColor) -> [u8; REPORT_LEN] {
    let mut r = new_report();
    set_header(&mut r, 0x05, 0x03, 0x01);
    r[8] = 0x01;
    r[9] = led as u8;
    r[10] = color.r;
    r[11] = color.g;
    r[12] = color.b;
    finalize(&mut r);
    r
}

pub fn set_led_state(led: LedId, on: bool) -> [u8; REPORT_LEN] {
    let mut r = new_report();
    set_header(&mut r, 0x03, 0x03, 0x00);
    r[8] = 0x01;
    r[9] = led as u8;
    r[10] = if on { 0x01 } else { 0x00 };
    finalize(&mut r);
    r
}

pub fn matrix_static(color: &RgbColor) -> [u8; REPORT_LEN] {
    let mut r = new_report();
    r[1] = 0xFF;
    set_header(&mut r, 0x09, 0x03, 0x0A);
    r[8] = 0x00;
    r[9] = 0x00;
    r[10] = 0x01;
    r[11] = 0x00;
    r[12] = 0x00;
    r[13] = color.r;
    r[14] = color.g;
    r[15] = color.b;
    finalize(&mut r);
    r
}

pub fn leds_for_zone(zone: &str) -> Vec<LedId> {
    match zone.to_ascii_lowercase().as_str() {
        "logo" => vec![LedId::Logo],
        "scroll" | "wheel" | "scroll_wheel" => vec![LedId::ScrollWheel],
        "backlight" | "strip" => vec![LedId::Backlight],
        "battery" => vec![LedId::Battery],
        "all" | "*" | "main" | "" => {
            vec![LedId::Logo, LedId::ScrollWheel, LedId::Backlight]
        }
        _ => vec![LedId::Logo, LedId::ScrollWheel],
    }
}
