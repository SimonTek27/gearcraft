//! Razer peripheral addon (skeleton).
//!
//! Protocol details vary by device generation (Chroma / Synapse). This module
//! registers known devices and exposes RGB zone stubs until open captures land.

use gearcraft_core::{DeviceInfo, PeripheralAddon, RgbColor};
use tracing::info;

/// USB vendor ID for Razer.
pub const RAZER_VID: u16 = 0x1532;

/// Try to build a Razer addon for a scanned device.
pub fn try_create(dev: &DeviceInfo) -> Option<Box<dyn PeripheralAddon>> {
    let name_l = dev.name.to_ascii_lowercase();
    let id_l = dev.id.to_ascii_lowercase();
    if !(name_l.contains("razer") || id_l.contains("razer") || id_l.contains("1532")) {
        return None;
    }
    Some(Box::new(RazerDevice { info: dev.clone() }))
}

struct RazerDevice {
    info: DeviceInfo,
}

impl PeripheralAddon for RazerDevice {
    fn device_info(&self) -> DeviceInfo {
        self.info.clone()
    }

    fn initialize(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!(
            "Razer addon init: {} ({:?})",
            self.info.name, self.info.kind
        );
        Ok(())
    }

    fn set_color(
        &mut self,
        zone: &str,
        color: RgbColor,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!(
            "Razer set_color [{}] zone={zone} rgb=({},{},{}) — HID write TBD",
            self.info.name, color.r, color.g, color.b
        );
        Ok(())
    }

    fn apply_profile(
        &mut self,
        profile: serde_json::Value,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("Razer apply_profile [{}]: {profile}", self.info.name);
        Ok(())
    }

    fn get_status(&self) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        Ok(serde_json::json!({
            "addon": "razer",
            "id": self.info.id,
            "name": self.info.name,
            "kind": format!("{:?}", self.info.kind),
            "rgb": "stub",
        }))
    }
}
