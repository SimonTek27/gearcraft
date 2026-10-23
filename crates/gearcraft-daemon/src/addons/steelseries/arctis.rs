//! Arctis headset status protocol (from Linux hid-steelseries).
//!
//! Battery request (Arctis 7 / 7 v2): output report `[0x06, 0x18]`
//! Connection query: `[0x06, 0x14]` — response byte2 `0x03` = connected
//! Battery level is in the response; over-reported values should be clamped to 100.

pub fn is_arctis_family(product_id: u16) -> bool {
    matches!(
        product_id,
        0x12AD | // Arctis 7 (2019)
        0x1260 | // Arctis 7 (2017)
        0x12BE | // Arctis 7+
        0x2202 | // Arctis Nova 7
        0x2206 // Arctis Nova 7x
    )
}

pub fn arctis_battery_request() -> Vec<u8> {
    vec![0x06, 0x18]
}

pub fn arctis_connection_request() -> Vec<u8> {
    vec![0x06, 0x14]
}

/// Parse a simplified battery response: data[0]==0x06, data[1]==0x18, data[2]=level
pub fn parse_battery_response(data: &[u8]) -> Option<u8> {
    if data.len() < 3 || data[0] != 0x06 {
        return None;
    }
    match data[1] {
        0x18 => Some(data[2].min(100)),
        _ => None,
    }
}
