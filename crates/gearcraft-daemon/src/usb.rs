//! USB / HID discovery helpers.
//!
//! Real device I/O lives in addons. This module enumerates candidates
//! (via hidapi when available, otherwise an empty list / demo mode).

use gearcraft_core::{DeviceInfo, DeviceKind};
use tracing::{debug, info, warn};

/// SteelSeries USB vendor ID
pub const VID_STEELSERIES: u16 = 0x1038;

/// Known SteelSeries product IDs we care about (expand over time).
pub const KNOWN_STEELSERIES: &[(u16, &str, DeviceKind)] = &[
    // Mice – Rival 3 family
    (0x1824, "Rival 3", DeviceKind::Mouse),
    (0x184C, "Rival 3 (fw 0.37)", DeviceKind::Mouse),
    (0x1830, "Rival 3 Wireless", DeviceKind::Mouse),
    (0x1870, "Rival 3 Gen 2", DeviceKind::Mouse),
    (0x1872, "Rival 3 Wireless Gen 2", DeviceKind::Mouse),
    // Keyboards – Apex
    (0x1612, "Apex 5", DeviceKind::Keyboard),
    (0x161A, "Apex 3", DeviceKind::Keyboard),
    (0x161C, "Apex 7", DeviceKind::Keyboard),
    (0x161E, "Apex 7 TKL", DeviceKind::Keyboard),
    (0x1622, "Apex 3 TKL", DeviceKind::Keyboard),
    // Headsets – Arctis
    (0x12AD, "Arctis 7", DeviceKind::Headset),
    (0x12BE, "Arctis 7+", DeviceKind::Headset),
    (0x1260, "Arctis 7 (2017)", DeviceKind::Headset),
    (0x2202, "Arctis Nova 7", DeviceKind::Headset),
];

/// Whether to skip real HID and use demo devices (env GEARCRAFT_DRY=1).
pub fn is_dry_run() -> bool {
    matches!(
        std::env::var("GEARCRAFT_DRY").as_deref(),
        Ok("1") | Ok("true") | Ok("yes")
    )
}

/// Scan for known devices. Without the `hidapi` feature this returns an empty vec
/// (caller may fall back to demo devices).
pub fn scan_devices() -> Vec<DeviceInfo> {
    if is_dry_run() {
        info!("GEARCRAFT_DRY=1 – skipping real USB scan");
        return Vec::new();
    }

    #[cfg(feature = "hidapi")]
    {
        match scan_with_hidapi() {
            Ok(list) => {
                info!("USB scan found {} device(s)", list.len());
                return list;
            }
            Err(e) => {
                warn!("hidapi scan failed: {e}");
            }
        }
    }

    #[cfg(not(feature = "hidapi"))]
    {
        debug!("hidapi feature disabled – no live USB scan");
    }

    Vec::new()
}

#[cfg(feature = "hidapi")]
fn scan_with_hidapi() -> Result<Vec<DeviceInfo>, Box<dyn std::error::Error + Send + Sync>> {
    use hidapi::HidApi;

    let api = HidApi::new()?;
    let mut out = Vec::new();

    for device in api.device_list() {
        if device.vendor_id() != VID_STEELSERIES {
            continue;
        }
        let pid = device.product_id();
        let known = KNOWN_STEELSERIES.iter().find(|(p, _, _)| *p == pid);
        let (name, kind) = match known {
            Some((_, n, k)) => ((*n).to_string(), k.clone()),
            None => (
                device
                    .product_string()
                    .unwrap_or("SteelSeries Device")
                    .to_string(),
                DeviceKind::Other,
            ),
        };

        // Prefer the control interface used by rivalcfg (often interface 3 for Rival 3)
        let iface = device.interface_number();
        let path = device.path().to_string_lossy().into_owned();
        let id = format!("ss-{:04x}-{:04x}-{}", VID_STEELSERIES, pid, iface);

        out.push(DeviceInfo {
            id,
            name,
            vendor: "SteelSeries".into(),
            kind,
            vendor_id: VID_STEELSERIES,
            product_id: pid,
            path: Some(path),
        });
    }

    // Deduplicate by product_id keeping the highest interface (control endpoint)
    out.sort_by(|a, b| {
        a.product_id
            .cmp(&b.product_id)
            .then_with(|| a.path.cmp(&b.path))
    });
    out.dedup_by(|a, b| a.product_id == b.product_id && a.vendor_id == b.vendor_id);

    Ok(out)
}

/// Open a HID device by path and write an output report.
/// Report data should **not** include the report-ID leading zero if the
/// device uses report ID 0; callers decide the payload layout.
#[cfg(feature = "hidapi")]
pub fn write_output_report(
    path: &str,
    data: &[u8],
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use hidapi::HidApi;

    let api = HidApi::new()?;
    let device = api.open_path(std::ffi::CString::new(path)?.as_c_str())?;
    // hidapi write: first byte is report ID (0x00 for most SteelSeries mice)
    let mut buf = Vec::with_capacity(data.len() + 1);
    buf.push(0x00);
    buf.extend_from_slice(data);
    device.write(&buf)?;
    Ok(())
}

#[cfg(not(feature = "hidapi"))]
pub fn write_output_report(
    _path: &str,
    data: &[u8],
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    debug!("write_output_report (stub): {:02x?}", data);
    Ok(())
}
