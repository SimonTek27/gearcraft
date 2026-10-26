//! USB / HID discovery helpers.
//!
//! Real device I/O lives in addons. This module enumerates candidates
//! (via hidapi when available, otherwise an empty list / demo mode).

use gearcraft_core::{DeviceInfo, DeviceKind};
#[allow(unused_imports)]
use tracing::{debug, info, warn};

/// SteelSeries USB vendor ID
pub const VID_STEELSERIES: u16 = 0x1038;

/// Razer USB vendor ID
#[allow(dead_code)]
pub const VID_RAZER: u16 = 0x1532;

/// Known SteelSeries product IDs we care about (expand over time).
/// Used by live HID scan (`feature = "hidapi"`).
#[cfg_attr(not(feature = "hidapi"), allow(dead_code))]
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
    (0x2206, "Arctis Nova 7x", DeviceKind::Headset),
];

/// Whether to skip real HID and use demo devices (env GEARCRAFT_DRY=1).
pub fn is_dry_run() -> bool {
    matches!(
        std::env::var("GEARCRAFT_DRY").as_deref(),
        Ok("1") | Ok("true") | Ok("yes")
    )
}

/// Scan-only diagnostic: list devices and exit (env GEARCRAFT_SCAN_ONLY=1).
pub fn is_scan_only() -> bool {
    matches!(
        std::env::var("GEARCRAFT_SCAN_ONLY").as_deref(),
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
fn preferred_interface(kind: &DeviceKind, iface: i32) -> i32 {
    // rivalcfg uses interface 3 for Rival 3 control; prefer higher iface for mice.
    // For other devices keep the reported interface as a score.
    match kind {
        DeviceKind::Mouse if iface >= 0 => iface,
        _ => iface.max(0),
    }
}

#[cfg(feature = "hidapi")]
fn scan_with_hidapi() -> Result<Vec<DeviceInfo>, Box<dyn std::error::Error + Send + Sync>> {
    use hidapi::HidApi;
    use std::collections::HashMap;

    let api = HidApi::new()?;
    // Collect all interfaces per (vid,pid), then pick the best one.
    let mut best: HashMap<(u16, u16), (i32, DeviceInfo)> = HashMap::new();

    for device in api.device_list() {
        let vid = device.vendor_id();
        let pid = device.product_id();
        let (vendor, name, kind) = if vid == VID_STEELSERIES {
            let known = KNOWN_STEELSERIES.iter().find(|(p, _, _)| *p == pid);
            match known {
                Some((_, n, k)) => ("SteelSeries".into(), (*n).to_string(), k.clone()),
                None => (
                    "SteelSeries".into(),
                    device
                        .product_string()
                        .unwrap_or("SteelSeries Device")
                        .to_string(),
                    DeviceKind::Other,
                ),
            }
        } else if vid == VID_RAZER {
            let known = crate::addons::razer::KNOWN_RAZER
                .iter()
                .find(|(p, _, _)| *p == pid);
            match known {
                Some((_, n, k)) => ("Razer".into(), (*n).to_string(), k.clone()),
                None => (
                    "Razer".into(),
                    device
                        .product_string()
                        .unwrap_or("Razer Device")
                        .to_string(),
                    DeviceKind::Other,
                ),
            }
        } else {
            continue;
        };

        let iface = device.interface_number();
        let path = device.path().to_string_lossy().into_owned();
        let prefix = if vid == VID_RAZER { "rz" } else { "ss" };
        let id = format!(
            "{prefix}-{:04x}-{:04x}-{}",
            vid,
            pid,
            if iface >= 0 { iface } else { 0 }
        );

        let info = DeviceInfo {
            id,
            name,
            vendor,
            kind: kind.clone(),
            vendor_id: vid,
            product_id: pid,
            path: Some(path),
        };

        let score = preferred_interface(&kind, iface);
        let key = (vid, pid);
        match best.get(&key) {
            Some((prev, _)) if *prev >= score => {}
            _ => {
                best.insert(key, (score, info));
            }
        }
    }

    let mut out: Vec<_> = best.into_values().map(|(_, d)| d).collect();
    out.sort_by(|a, b| {
        a.product_id
            .cmp(&b.product_id)
            .then_with(|| a.name.cmp(&b.name))
    });
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
    let n = device.write(&buf)?;
    debug!("HID write {n} bytes to {path}: {:02x?}", data);
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

/// Write a HID **feature** report (Razer control frames, etc.).
/// `data` is the full report body; report ID 0 is prepended for hidapi.
#[cfg(feature = "hidapi")]
pub fn write_feature_report(
    path: &str,
    data: &[u8],
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use hidapi::HidApi;

    let api = HidApi::new()?;
    let device = api.open_path(std::ffi::CString::new(path)?.as_c_str())?;
    let mut buf = Vec::with_capacity(data.len() + 1);
    buf.push(0x00);
    buf.extend_from_slice(data);
    device.send_feature_report(&buf)?;
    debug!("HID feature write {} bytes to {path}: {:02x?}", data.len(), &data[..data.len().min(16)]);
    Ok(())
}

#[cfg(not(feature = "hidapi"))]
pub fn write_feature_report(
    _path: &str,
    data: &[u8],
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    debug!("write_feature_report (stub): {:02x?}", data);
    Ok(())
}


/// Write an output report then try to read a response (for battery / status queries).
#[cfg(feature = "hidapi")]
pub fn write_then_read(
    path: &str,
    data: &[u8],
    timeout_ms: i32,
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    use hidapi::HidApi;

    let api = HidApi::new()?;
    let device = api.open_path(std::ffi::CString::new(path)?.as_c_str())?;
    let mut buf = Vec::with_capacity(data.len() + 1);
    buf.push(0x00);
    buf.extend_from_slice(data);
    device.write(&buf)?;

    let mut resp = vec![0u8; 64];
    let n = device.read_timeout(&mut resp, timeout_ms)?;
    if n == 0 {
        return Err("HID read timeout".into());
    }
    resp.truncate(n);
    debug!("HID read {n} bytes from {path}: {:02x?}", &resp);
    Ok(resp)
}

#[cfg(not(feature = "hidapi"))]
pub fn write_then_read(
    _path: &str,
    data: &[u8],
    _timeout_ms: i32,
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    debug!("write_then_read (stub): {:02x?}", data);
    // Demo battery-like response for Arctis query
    if data == [0x06, 0x18] {
        return Ok(vec![0x06, 0x18, 78]);
    }
    Ok(Vec::new())
}

/// Print a human-readable scan report (used by GEARCRAFT_SCAN_ONLY).
pub fn print_scan_report() {
    println!("GearCraft HID scan");
    println!("  feature hidapi: {}", cfg!(feature = "hidapi"));
    println!("  dry-run:        {}", is_dry_run());
    let list = scan_devices();
    if list.is_empty() {
        println!("  devices:        (none)");
        println!();
        println!("Hints:");
        println!("  • Live HID is built in (feature: {})", cfg!(feature = "hidapi"));
        println!("  • udev rules:  sudo cp assets/udev/99-gearcraft.rules /etc/udev/rules.d/");
        println!("  • group:       sudo usermod -aG plugdev $USER  (then re-login)");
        println!("  • list raw:    lsusb -d 1038:   # SteelSeries");
        println!("  • demo only:   GEARCRAFT_DRY=1");
        return;
    }
    println!("  devices:        {}", list.len());
    for d in &list {
        println!(
            "  - {}  [{:04x}:{:04x}]  kind={:?}  id={}  path={}",
            d.name,
            d.vendor_id,
            d.product_id,
            d.kind,
            d.id,
            d.path.as_deref().unwrap_or("-")
        );
    }
}
