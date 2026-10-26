//! Official / system driver backends (preferred over raw HID).
//!
//! - **OpenRazer** (`org.razer` D-Bus) — Razer mice/keyboards/headsets
//! - **OpenRGB** (CLI / SDK port) — multi-vendor RGB including SteelSeries

pub mod openrazer;
pub mod openrgb;
pub mod ratbag;

use gearcraft_core::RgbColor;
use tracing::info;

/// Try official backends first; returns Ok(true) if handled.
pub async fn set_color_official(
    vendor: &str,
    product_hint: &str,
    zone: &str,
    color: RgbColor,
) -> Result<bool, String> {
    let vendor_l = vendor.to_ascii_lowercase();
    if vendor_l.contains("razer") {
        if openrazer::available().await {
            openrazer::set_color(product_hint, zone, color).await?;
            info!("RGB via OpenRazer D-Bus ({product_hint} / {zone})");
            return Ok(true);
        }
    }
    if openrgb::available().await {
        if openrgb::set_color(product_hint, zone, color).await? {
            info!("RGB via OpenRGB ({product_hint})");
            return Ok(true);
        }
    }
    Ok(false)
}

pub async fn set_effect_official(
    vendor: &str,
    product_hint: &str,
    zone: &str,
    effect: &str,
    color: Option<RgbColor>,
) -> Result<bool, String> {
    let vendor_l = vendor.to_ascii_lowercase();
    if vendor_l.contains("razer") && openrazer::available().await {
        openrazer::set_effect(product_hint, zone, effect, color).await?;
        info!("Effect via OpenRazer D-Bus ({effect})");
        return Ok(true);
    }
    if openrgb::available().await {
        if openrgb::set_effect(product_hint, effect, color).await? {
            info!("Effect via OpenRGB ({effect})");
            return Ok(true);
        }
    }
    Ok(false)
}


/// Apply effect/color to every OpenRGB device (and Razer via OpenRazer if up).
pub async fn prism_sync(
    effect: &str,
    zone: &str,
    color: Option<gearcraft_core::RgbColor>,
) -> Result<serde_json::Value, String> {
    let mut results = Vec::new();
    // OpenRGB: paint all listed devices
    if openrgb::available().await {
        if let Ok(list) = openrgb::list_devices().await {
            for line in list {
                let hint = line.split(':').nth(1).unwrap_or(&line).trim();
                let color = color.clone();
                match openrgb::set_effect(hint, effect, color).await {
                    Ok(true) => results.push(serde_json::json!({
                        "device": hint, "backend": "openrgb", "ok": true
                    })),
                    Ok(false) => {}
                    Err(e) => results.push(serde_json::json!({
                        "device": hint, "backend": "openrgb", "ok": false, "error": e
                    })),
                }
            }
        }
    }
    // OpenRazer: each serial
    if openrazer::available().await {
        if let Ok(serials) = openrazer::list_serials().await {
            for ser in serials {
                let color = color.clone();
                match openrazer::set_effect(&ser, zone, effect, color).await {
                    Ok(()) => results.push(serde_json::json!({
                        "device": ser, "backend": "openrazer", "ok": true
                    })),
                    Err(e) => results.push(serde_json::json!({
                        "device": ser, "backend": "openrazer", "ok": false, "error": e
                    })),
                }
            }
        }
    }
    if results.is_empty() {
        return Err(
            "PrismSync: no OpenRGB/OpenRazer devices (start openrgb --server and/or openrazer-daemon)"
                .into(),
        );
    }
    Ok(serde_json::json!({ "synced": results.len(), "results": results }))
}
