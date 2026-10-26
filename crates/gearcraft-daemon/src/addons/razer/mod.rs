//! Razer peripheral addon — RGB via OpenRazer-compatible HID reports.

mod protocol;

use gearcraft_core::{DeviceInfo, DeviceKind, PeripheralAddon, RgbColor};
use tracing::{debug, info, warn};

use crate::usb;
use protocol::{leds_for_zone, matrix_static, set_led_effect, set_led_rgb, set_led_state, Effect, LedId};

pub const RAZER_VID: u16 = 0x1532;

#[allow(dead_code)]
pub const KNOWN_RAZER: &[(u16, &str, DeviceKind)] = &[
    (0x0043, "Abyssus", DeviceKind::Mouse),
    (0x005C, "DeathAdder Essential", DeviceKind::Mouse),
    (0x0060, "DeathAdder Elite", DeviceKind::Mouse),
    (0x006E, "Basilisk", DeviceKind::Mouse),
    (0x006C, "DeathAdder Elite (alt)", DeviceKind::Mouse),
    (0x0078, "Viper", DeviceKind::Mouse),
    (0x0083, "DeathAdder V2 Pro", DeviceKind::Mouse),
    (0x0084, "DeathAdder V2", DeviceKind::Mouse),
    (0x0086, "Basilisk Ultimate", DeviceKind::Mouse),
    (0x0088, "Basilisk X HyperSpeed", DeviceKind::Mouse),
    (0x008A, "Viper Ultimate", DeviceKind::Mouse),
    (0x0098, "Naga Pro", DeviceKind::Mouse),
    (0x00A1, "DeathAdder V2 Mini", DeviceKind::Mouse),
    (0x00A7, "Basilisk V3", DeviceKind::Mouse),
    (0x00B7, "DeathAdder V3", DeviceKind::Mouse),
    (0x00B6, "DeathAdder V3 Pro", DeviceKind::Mouse),
    (0x0221, "BlackWidow Chroma", DeviceKind::Keyboard),
    (0x022A, "BlackWidow Chroma V2", DeviceKind::Keyboard),
    (0x0241, "BlackWidow Elite", DeviceKind::Keyboard),
    (0x0255, "BlackWidow V3", DeviceKind::Keyboard),
    (0x025A, "BlackWidow V3 TKL", DeviceKind::Keyboard),
    (0x028A, "BlackWidow V3 Mini", DeviceKind::Keyboard),
    (0x0C00, "Firefly", DeviceKind::Other),
    (0x0C04, "Goliathus", DeviceKind::Other),
];

pub fn try_create(dev: &DeviceInfo) -> Option<Box<dyn PeripheralAddon>> {
    let name_l = dev.name.to_ascii_lowercase();
    let id_l = dev.id.to_ascii_lowercase();
    let is_razer = dev.vendor_id == RAZER_VID
        || name_l.contains("razer")
        || id_l.contains("razer")
        || id_l.contains("1532");
    if !is_razer {
        return None;
    }
    Some(Box::new(RazerDevice {
        info: dev.clone(),
        initialized: false,
    }))
}

struct RazerDevice {
    info: DeviceInfo,
    initialized: bool,
}

impl RazerDevice {
    fn send_report(
        &self,
        report: &[u8],
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let path = self
            .info
            .path
            .as_deref()
            .ok_or("Razer device has no HID path (demo or missing permissions)")?;
        usb::write_feature_report(path, report)?;
        debug!(
            "Razer feature report → {} (class={:02x} cmd={:02x})",
            self.info.name,
            report.get(6).copied().unwrap_or(0),
            report.get(7).copied().unwrap_or(0),
        );
        Ok(())
    }

    fn apply_static_color(
        &self,
        zone: &str,
        color: &RgbColor,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        match self.info.kind {
            DeviceKind::Keyboard | DeviceKind::Other => {
                self.send_report(&matrix_static(color))?;
            }
            DeviceKind::Mouse | _ => {
                for led in leds_for_zone(zone) {
                    let _ = self.send_report(&set_led_state(led, true));
                    let _ = self.send_report(&set_led_effect(led, Effect::Static));
                    self.send_report(&set_led_rgb(led, color))?;
                }
            }
        }
        Ok(())
    }
}

impl PeripheralAddon for RazerDevice {
    fn device_info(&self) -> DeviceInfo {
        self.info.clone()
    }

    fn initialize(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!(
            "Razer addon init: {} [{:04x}:{:04x}] path={}",
            self.info.name,
            self.info.vendor_id,
            self.info.product_id,
            self.info.path.as_deref().unwrap_or("-")
        );
        if self.info.path.is_some() {
            for led in [LedId::Logo, LedId::ScrollWheel] {
                if let Err(e) = self.send_report(&set_led_state(led, true)) {
                    debug!("Razer init led {:?}: {e}", led);
                }
            }
        } else {
            warn!(
                "Razer {}: no HID path – RGB writes need hidapi + udev",
                self.info.name
            );
        }
        self.initialized = true;
        Ok(())
    }

    fn set_color(
        &mut self,
        zone: &str,
        color: RgbColor,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if !self.initialized {
            self.initialize()?;
        }
        info!(
            "Razer set_color [{}] zone={zone} rgb=({},{},{})",
            self.info.name, color.r, color.g, color.b
        );
        self.apply_static_color(zone, &color)
    }

    fn apply_profile(
        &mut self,
        profile: serde_json::Value,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if !self.initialized {
            self.initialize()?;
        }
        if let Some(colors) = profile.get("colors").and_then(|v| v.as_object()) {
            for (zone, val) in colors {
                let r = val.get("r").and_then(|v| v.as_u64()).unwrap_or(0) as u8;
                let g = val.get("g").and_then(|v| v.as_u64()).unwrap_or(0) as u8;
                let b = val.get("b").and_then(|v| v.as_u64()).unwrap_or(0) as u8;
                self.set_color(zone, RgbColor { r, g, b })?;
            }
        }
        if let Some(c) = profile.get("color") {
            let r = c.get("r").and_then(|v| v.as_u64()).unwrap_or(0) as u8;
            let g = c.get("g").and_then(|v| v.as_u64()).unwrap_or(0) as u8;
            let b = c.get("b").and_then(|v| v.as_u64()).unwrap_or(0) as u8;
            self.set_color("all", RgbColor { r, g, b })?;
        }
        Ok(())
    }

    fn get_status(&self) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        Ok(serde_json::json!({
            "addon": "razer",
            "id": self.info.id,
            "name": self.info.name,
            "kind": format!("{:?}", self.info.kind),
            "vendor_id": self.info.vendor_id,
            "product_id": self.info.product_id,
            "path": self.info.path,
            "rgb": "openrazer-compat",
            "zones": ["logo", "scroll", "backlight", "all"],
        }))
    }
}
