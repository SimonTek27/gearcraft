//! Apex keyboard RGB – simplified direct-colour report.
//!
//! Based on community captures (Apex 3 / Apex 3 TKL / AntiBloatLight):
//! ```text
//! 21 FF  <R G B> × N_zones  …
//! ```
//! Full per-key protocols differ by model; this sets a uniform colour
//! across the backlight zones exposed by the simplified path.

use gearcraft_core::RgbColor;

pub fn is_apex_family(product_id: u16) -> bool {
    matches!(
        product_id,
        0x1612 | // Apex 5
        0x161A | // Apex 3
        0x161C | // Apex 7
        0x161E | // Apex 7 TKL
        0x1622 | // Apex 3 TKL
        0x1620 // Apex Pro variants (subset)
    )
}

/// Number of RGB slots to fill in the direct report (10 zones is common on Apex 3).
const ZONE_COUNT: usize = 10;

/// Build a direct-colour output report (without report-ID leading 0x00).
pub fn apex_direct_color_report(color: &RgbColor) -> Vec<u8> {
    // Opcode 0x21, flag 0xFF (all / direct), then RGB triplets
    let mut buf = vec![0x21, 0xFF];
    for _ in 0..ZONE_COUNT {
        buf.push(color.r);
        buf.push(color.g);
        buf.push(color.b);
    }
    // Pad to a typical 64-byte HID payload tail
    while buf.len() < 64 {
        buf.push(0);
    }
    buf
}
