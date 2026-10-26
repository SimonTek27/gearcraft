//! OpenRazer daemon backend via D-Bus (`org.razer`).
//!
//! Requires `openrazer-daemon` (and typically `openrazer-driver-dkms`).
//! Uses `busctl --user` so we don't need a zbus dependency.

use std::process::Stdio;

use gearcraft_core::RgbColor;
use tokio::process::Command;
use tracing::{debug, warn};

const DEST: &str = "org.razer";

pub async fn available() -> bool {
    let out = Command::new("busctl")
        .args(["--user", "status", DEST])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await;
    matches!(out, Ok(s) if s.success())
}

async fn busctl_call(args: &[&str]) -> Result<String, String> {
    let out = Command::new("busctl")
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|e| format!("busctl: {e}"))?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        return Err(format!("busctl failed: {err}"));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// List device serials from OpenRazer.
pub async fn list_serials() -> Result<Vec<String>, String> {
    // busctl --user call org.razer /org/razer/devices razer.devices getDevices
    let out = busctl_call(&[
        "--user",
        "call",
        DEST,
        "/org/razer/devices",
        "razer.devices",
        "getDevices",
    ])
    .await?;
    // Output like: as 2 "SERIAL1" "SERIAL2"  or  as 0
    parse_busctl_string_array(&out)
}

fn parse_busctl_string_array(s: &str) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    // Prefer quoted strings
    let mut in_quote = false;
    let mut cur = String::new();
    for c in s.chars() {
        match c {
            '"' if !in_quote => {
                in_quote = true;
                cur.clear();
            }
            '"' if in_quote => {
                in_quote = false;
                if !cur.is_empty() {
                    out.push(cur.clone());
                }
                cur.clear();
            }
            c if in_quote => cur.push(c),
            _ => {}
        }
    }
    Ok(out)
}

fn device_path(serial: &str) -> String {
    // OpenRazer uses /org/razer/device/{serial} — serial may need encoding
    format!("/org/razer/device/{serial}")
}

async fn resolve_serial(product_hint: &str) -> Result<String, String> {
    let serials = list_serials().await?;
    if serials.is_empty() {
        return Err("OpenRazer: no devices (is openrazer-daemon running?)".into());
    }
    if serials.len() == 1 {
        return Ok(serials[0].clone());
    }
    // Match by getDeviceName
    let hint = product_hint.to_ascii_lowercase();
    for ser in &serials {
        let path = device_path(ser);
        if let Ok(name) = busctl_call(&[
            "--user",
            "call",
            DEST,
            &path,
            "razer.device.misc",
            "getDeviceName",
        ])
        .await
        {
            let name_l = name.to_ascii_lowercase();
            if name_l.contains(&hint)
                || hint.contains(name_l.trim_matches('"').split_whitespace().next().unwrap_or(""))
            {
                return Ok(ser.clone());
            }
        }
    }
    // Fallback first device
    debug!("OpenRazer: no name match for «{product_hint}», using first serial");
    Ok(serials[0].clone())
}

pub async fn set_color(product_hint: &str, zone: &str, color: RgbColor) -> Result<(), String> {
    let serial = resolve_serial(product_hint).await?;
    let path = device_path(&serial);
    let zone = zone.to_ascii_lowercase();
    let (iface, method) = match zone.as_str() {
        "logo" => ("razer.device.lighting.logo", "setLogoStatic"),
        "scroll" | "scroll_wheel" => ("razer.device.lighting.scroll", "setScrollStatic"),
        "backlight" => ("razer.device.lighting.backlight", "setBacklightStatic"),
        "all" | _ => ("razer.device.lighting.chroma", "setStatic"),
    };
    // yyy = three bytes
    let r = color.r.to_string();
    let g = color.g.to_string();
    let b = color.b.to_string();
    match busctl_call(&[
        "--user",
        "call",
        DEST,
        &path,
        iface,
        method,
        "yyy",
        &r,
        &g,
        &b,
    ])
    .await
    {
        Ok(_) => Ok(()),
        Err(e) => {
            // Chroma setStatic often works when zone-specific fails
            if iface != "razer.device.lighting.chroma" {
                warn!("OpenRazer {method} failed ({e}), trying setStatic");
                busctl_call(&[
                    "--user",
                    "call",
                    DEST,
                    &path,
                    "razer.device.lighting.chroma",
                    "setStatic",
                    "yyy",
                    &r,
                    &g,
                    &b,
                ])
                .await?;
                return Ok(());
            }
            Err(e)
        }
    }
}

pub async fn set_effect(
    product_hint: &str,
    zone: &str,
    effect: &str,
    color: Option<RgbColor>,
) -> Result<(), String> {
    let serial = resolve_serial(product_hint).await?;
    let path = device_path(&serial);
    let effect = effect.to_ascii_lowercase();
    let zone = zone.to_ascii_lowercase();

    // Prefer chroma interface for device-wide effects
    match effect.as_str() {
        "static" => {
            let c = color.unwrap_or(RgbColor {
                r: 0,
                g: 231,
                b: 160,
            });
            return set_color(product_hint, &zone, c).await;
        }
        "spectrum" | "rainbow" => {
            try_effect_methods(
                &path,
                &zone,
                &[
                    ("razer.device.lighting.chroma", "setSpectrum"),
                    ("razer.device.lighting.logo", "setLogoSpectrum"),
                    ("razer.device.lighting.scroll", "setScrollSpectrum"),
                ],
            )
            .await
        }
        "breathe" | "breathing" => {
            if let Some(c) = color {
                let r = c.r.to_string();
                let g = c.g.to_string();
                let b = c.b.to_string();
                // setBreathSingle yyy
                return busctl_call(&[
                    "--user",
                    "call",
                    DEST,
                    &path,
                    "razer.device.lighting.chroma",
                    "setBreathSingle",
                    "yyy",
                    &r,
                    &g,
                    &b,
                ])
                .await
                .map(|_| ());
            }
            try_effect_methods(
                &path,
                &zone,
                &[("razer.device.lighting.chroma", "setBreathRandom")],
            )
            .await
        }
        "wave" => {
            // setWave i — direction 1 or 2
            busctl_call(&[
                "--user",
                "call",
                DEST,
                &path,
                "razer.device.lighting.chroma",
                "setWave",
                "i",
                "1",
            ])
            .await
            .map(|_| ())
        }
        "off" | "none" => {
            try_effect_methods(
                &path,
                &zone,
                &[
                    ("razer.device.lighting.chroma", "setNone"),
                    ("razer.device.lighting.logo", "setLogoNone"),
                    ("razer.device.lighting.scroll", "setScrollNone"),
                ],
            )
            .await
        }
        _ => {
            if let Some(c) = color {
                set_color(product_hint, &zone, c).await
            } else {
                Err(format!("OpenRazer: unknown effect {effect}"))
            }
        }
    }
}

async fn try_effect_methods(
    path: &str,
    _zone: &str,
    methods: &[(&str, &str)],
) -> Result<(), String> {
    let mut last = String::new();
    for (iface, method) in methods {
        match busctl_call(&["--user", "call", DEST, path, iface, method]).await {
            Ok(_) => return Ok(()),
            Err(e) => last = e,
        }
    }
    Err(last)
}
