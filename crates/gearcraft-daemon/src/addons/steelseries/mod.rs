//! SteelSeries addon – protocols derived from rivalcfg, OpenRGB research, and kernel hid-steelseries.
//!
//! | Family   | Protocol module | Capabilities              |
//! |----------|-----------------|---------------------------|
//! | Rival 3  | rival3          | RGB zones, polling, effect|
//! | Apex 3/5 | apex            | Multi-zone keyboard RGB   |
//! | Arctis 7 | arctis          | Battery / connection query|

mod apex;
mod arctis;
mod rival3;

use gearcraft_core::{DeviceInfo, DeviceKind, PeripheralAddon, RgbColor};
use tracing::{info, warn};

use crate::usb;

pub use apex::{apex_direct_color_report, is_apex_family};
pub use arctis::{arctis_battery_request, is_arctis_family};
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

    pub fn set_zone_color(
        &self,
        zone: &str,
        color: &RgbColor,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if is_rival3_family(self.info.product_id) {
            let z = zone_id_rival3(zone).unwrap_or(0);
            let report = rival3_color_report(z, color.r, color.g, color.b);
            self.send(&report)?;
            let _ = self.send(&rival3_save_report());
            return Ok(());
        }
        if is_apex_family(self.info.product_id) {
            let report = apex_direct_color_report(color.r, color.g, color.b);
            return self.send(&report);
        }
        warn!("set_zone_color: unsupported PID {:04x}", self.info.product_id);
        Ok(())
    }
}

impl PeripheralAddon for SteelSeriesAddon {
    fn name(&self) -> &str {
        &self.info.name
    }

    fn vendor_id(&self) -> u16 {
        self.info.vendor_id
    }

    fn supported_pids(&self) -> &[u16] {
        &[]
    }

    fn probe(&self) -> Result<Vec<DeviceInfo>, Box<dyn std::error::Error + Send + Sync>> {
        Ok(vec![self.info.clone()])
    }

    fn set_color(
        &self,
        _device: &DeviceInfo,
        color: &RgbColor,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.set_zone_color("all", color)
    }

    fn set_dpi(
        &self,
        _device: &DeviceInfo,
        _stages: &[gearcraft_core::DpiStage],
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Ok(())
    }
}

/// Build demo devices when no real HID is available.
pub fn demo_devices() -> Vec<DeviceInfo> {
    vec![
        DeviceInfo {
            id: "demo-rival3".into(),
            name: "Rival 3 (demo)".into(),
            vendor: "SteelSeries".into(),
            kind: DeviceKind::Mouse,
            vendor_id: 0x1038,
            product_id: 0x1824,
            path: None,
        },
        DeviceInfo {
            id: "demo-apex".into(),
            name: "Apex 3 (demo)".into(),
            vendor: "SteelSeries".into(),
            kind: DeviceKind::Keyboard,
            vendor_id: 0x1038,
            product_id: 0x161A,
            path: None,
        },
        DeviceInfo {
            id: "demo-arctis".into(),
            name: "Arctis 7 (demo)".into(),
            vendor: "SteelSeries".into(),
            kind: DeviceKind::Headset,
            vendor_id: 0x1038,
            product_id: 0x12AD,
            path: None,
        },
    ]
}
