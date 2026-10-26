//! Rival 3 family HID protocol (from rivalcfg).

use gearcraft_core::RgbColor;

pub fn is_rival3_family(product_id: u16) -> bool {
    matches!(product_id, 0x1824 | 0x184C | 0x1830 | 0x1870 | 0x1872)
}

pub fn rival3_color_report(led_id: u8, color: &RgbColor, brightness: u8) -> Vec<u8> {
    let br = brightness.min(0x64);
    vec![0x05, 0x00, led_id, color.r, color.g, color.b, br]
}

pub fn rival3_polling_report(hz: u16) -> Option<Vec<u8>> {
    let code = match hz {
        125 => 0x04u8,
        250 => 0x03,
        500 => 0x02,
        1000 => 0x01,
        _ => return None,
    };
    Some(vec![0x04, 0x00, code])
}

pub fn rival3_save_report() -> Vec<u8> {
    vec![0x09, 0x00]
}

pub fn rival3_effect_report(effect: &str) -> Option<Vec<u8>> {
    let code = match effect {
        "rainbow-shift" => 0x00u8,
        "breath-fast" => 0x01,
        "breath" => 0x02,
        "breath-slow" => 0x03,
        "steady" => 0x04,
        "rainbow-breath" => 0x05,
        "disco" => 0x06,
        _ => return None,
    };
    Some(vec![0x06, 0x00, code])
}
