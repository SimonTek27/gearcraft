//! SteelSeries addon – protocols derived from rivalcfg, OpenRGB research, and kernel hid-steelseries.
//!
//! | Family   | Protocol module | Capabilities              |
//! |----------|-----------------|---------------------------|
//! | Rival 3/5| rival3          | RGB zones, polling, effect|
//! | Apex 3/5 | apex            | Multi-zone keyboard RGB   |
//! | Arctis 7 | arctis          | Battery / connection query|

mod apex;
mod arctis;
mod rival3;

use gearcraft_core::{DeviceInfo, DeviceKind, PeripheralAddon, RgbColor};
use tracing::{debug, info, warn};

use crate::usb;

pub use apex::{apex_direct_color_report, is_apex_family};
pub use arctis::{arctis_battery_request, arctis_connection_request, is_arctis_family, parse_battery_response};
pub use rival3::{
    is_rival3_family, rival3_color_report, rival3_effect_report, rival3_polling_report,
    rival3_save_report,
};

fn zone_id_rival3(zone: &str) -> Option<u8> {
    match zone.to_ascii_lowercase().as_str() {
        "all" | "*" => Some(0x00),
        "z1" | "top" | "strip-top" => Some(0x01),
        "z2" | "middle" | "strip-middle" => Some(0x02),
        "z3" | "bottom" | "strip-bottom" => Some(0x03),
        "z4" | "logo" => Some(0x04),
        _ => None,
    }
}

pub struct SteelSeriesAddon {
    info: DeviceInfo,
    path: Option<String>,
    initialized: bool,
    /// Last known battery % for headsets (demo or live)
    battery: Option<u8>,
}

impl SteelSeriesAddon {
    pub fn new(info: DeviceInfo) -> Self {
        let path = info.path.clone();
        let battery = if info.kind == DeviceKind::Headset {
            Some(78)
        } else {
            None
        };
        Self {
            info,
            path,
            initialized: false,
            battery,
        }
    }

    fn protocol_name(&self) -> &'static str {
        if is_rival3_family(self.info.product_id) {
            "rival3"
        } else if is_apex_family(self.info.product_id) {
            "apex"
        } else if is_arctis_family(self.info.product_id) {
            "arctis"
        } else {
            "generic"
        }
    }

    fn send(&self, payload: &[u8]) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let path = self.path.as_deref().ok_or("no HID path for device")?;
        info!(
            "SteelSeries [{}] → {}  report {:02x?}",
            self.protocol_name(),
            self.info.name,
            payload
        );
        usb::write_output_report(path, payload)
    }
}

impl PeripheralAddon for SteelSeriesAddon {
    fn device_info(&self) -> DeviceInfo {
        self.info.clone()
    }

    fn initialize(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!(
            "Initializing SteelSeries/{} for {}",
            self.protocol_name(),
            self.info.name
        );
        if is_arctis_family(self.info.product_id) {
            // Best-effort battery poll (write + read when hidapi is available)
            if let Some(path) = self.path.as_deref() {
                match usb::write_then_read(path, &arctis_battery_request(), 500) {
                    Ok(resp) => {
                        if let Some(level) = parse_battery_response(&resp) {
                            info!("{} battery {}%", self.info.name, level);
                            self.battery = Some(level);
                        } else {
                            warn!("{} battery parse failed: {:02x?}", self.info.name, resp);
                        }
                    }
                    Err(e) => warn!("{} battery query: {e}", self.info.name),
                }
                // Connection probe (best-effort; response not always available)
                match usb::write_then_read(path, &arctis_connection_request(), 300) {
                    Ok(resp) => {
                        info!("{} connection query resp {:02x?}", self.info.name, resp);
                    }
                    Err(e) => {
                        debug!("{} connection query: {e}", self.info.name);
                    }
                }
            }
        }
        self.initialized = true;
        Ok(())
    }

    fn apply_profile(
        &mut self,
        profile_data: serde_json::Value,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if !self.initialized {
            self.initialize()?;
        }

        if is_rival3_family(self.info.product_id) {
            if let Some(effect) = profile_data.get("effect").and_then(|v| v.as_str()) {
                if let Some(report) = rival3_effect_report(effect) {
                    self.send(&report)?;
                }
            }
            if let Some(hz) = profile_data.get("polling_rate").and_then(|v| v.as_u64()) {
                if let Some(report) = rival3_polling_report(hz as u16) {
                    self.send(&report)?;
                }
            }
        }

        if let Some(colors) = profile_data.get("colors").and_then(|v| v.as_object()) {
            for (zone, val) in colors {
                let r = val.get("r").and_then(|v| v.as_u64()).unwrap_or(0) as u8;
                let g = val.get("g").and_then(|v| v.as_u64()).unwrap_or(0) as u8;
                let b = val.get("b").and_then(|v| v.as_u64()).unwrap_or(0) as u8;
                self.set_color(zone, RgbColor { r, g, b })?;
            }
        }

        // Single flat colour for keyboards
        if let Some(c) = profile_data.get("color") {
            let r = c.get("r").and_then(|v| v.as_u64()).unwrap_or(0) as u8;
            let g = c.get("g").and_then(|v| v.as_u64()).unwrap_or(0) as u8;
            let b = c.get("b").and_then(|v| v.as_u64()).unwrap_or(0) as u8;
            self.set_color("all", RgbColor { r, g, b })?;
        }

        if is_rival3_family(self.info.product_id)
            && profile_data
                .get("save")
                .and_then(|v| v.as_bool())
                .unwrap_or(true)
        {
            self.send(&rival3_save_report())?;
        }

        Ok(())
    }

    fn get_status(&self) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        Ok(serde_json::json!({
            "addon": "steelseries",
            "protocol": self.protocol_name(),
            "initialized": self.initialized,
            "path": self.path,
            "product_id": format!("0x{:04X}", self.info.product_id),
            "battery": self.battery,
        }))
    }

    fn set_color(
        &mut self,
        zone: &str,
        color: RgbColor,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if is_rival3_family(self.info.product_id) {
            let led = zone_id_rival3(zone).ok_or_else(|| format!("unknown zone: {zone}"))?;
            self.send(&rival3_color_report(led, &color, 0x64))?;
            return Ok(());
        }

        if is_apex_family(self.info.product_id) {
            // Apex: paint all zones (or "all") with the same colour for now
            self.send(&apex_direct_color_report(&color))?;
            return Ok(());
        }

        if is_arctis_family(self.info.product_id) {
            warn!("Arctis devices have no RGB zones via this protocol");
            return Ok(());
        }

        warn!(
            "set_color: unknown protocol for PID 0x{:04X} – sending rival3-style report",
            self.info.product_id
        );
        let led = zone_id_rival3(zone).unwrap_or(0x00);
        self.send(&rival3_color_report(led, &color, 0x64))?;
        Ok(())
    }
}

pub fn try_create(info: &DeviceInfo) -> Option<Box<dyn PeripheralAddon>> {
    if info.vendor_id != usb::VID_STEELSERIES {
        return None;
    }
    Some(Box::new(SteelSeriesAddon::new(info.clone())))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rival3_logo_report() {
        let c = RgbColor {
            r: 0,
            g: 231,
            b: 160,
        };
        assert_eq!(
            rival3_color_report(0x04, &c, 100),
            vec![0x05, 0x00, 0x04, 0, 231, 160, 0x64]
        );
    }

    #[test]
    fn apex_report_len() {
        let c = RgbColor {
            r: 255,
            g: 0,
            b: 0,
        };
        let r = apex_direct_color_report(&c);
        assert!(r.len() >= 10);
        assert_eq!(r[0], 0x21);
    }

    /// Every PID advertised in the known-device table must be handled by a
    /// protocol family, otherwise live detection would fall back to generic.
    #[test]
    fn known_pids_have_a_protocol_family() {
        for (pid, name, _kind) in crate::usb::KNOWN_STEELSERIES {
            assert!(
                is_rival3_family(*pid) || is_apex_family(*pid) || is_arctis_family(*pid),
                "{name} (0x{pid:04X}) has no protocol family"
            );
        }
    }

    #[test]
    fn rival5_is_rival3_family() {
        assert!(is_rival3_family(0x183C));
        assert_eq!(
            rival3_color_report(0x00, &RgbColor { r: 1, g: 2, b: 3 }, 100),
            vec![0x05, 0x00, 0x00, 1, 2, 3, 0x64]
        );
    }
}
