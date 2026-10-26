//! OpenRGB backend — multi-vendor RGB (SteelSeries, Razer, Corsair, …).
//!
//! Prefers the `openrgb` CLI when the OpenRGB SDK server is running
//! (default port 6742).

use std::process::Stdio;

use gearcraft_core::RgbColor;
use tokio::process::Command;
use tracing::debug;

pub async fn available() -> bool {
    // openrgb CLI present and optionally server reachable
    let cli = Command::new("openrgb")
        .arg("--help")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await;
    matches!(cli, Ok(s) if s.success() || s.code().is_some())
}

async fn run_openrgb(args: &[&str]) -> Result<String, String> {
    let out = Command::new("openrgb")
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|e| format!("openrgb: {e}"))?;
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    if !out.status.success() {
        return Err(format!("openrgb failed: {stderr} {stdout}"));
    }
    Ok(stdout)
}

/// List devices: returns lines from `openrgb -l`
pub async fn list_devices() -> Result<Vec<String>, String> {
    let out = run_openrgb(&["-l"]).await?;
    Ok(out
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect())
}

fn parse_device_index(list_line: &str) -> Option<u32> {
    // Typical: "0: Device Name"
    let num = list_line.split(':').next()?.trim();
    num.parse().ok()
}

async fn find_device_index(product_hint: &str) -> Result<Option<u32>, String> {
    let lines = list_devices().await?;
    if lines.is_empty() {
        return Ok(None);
    }
    let hint = product_hint.to_ascii_lowercase();
    for line in &lines {
        if line.to_ascii_lowercase().contains(&hint) {
            if let Some(i) = parse_device_index(line) {
                return Ok(Some(i));
            }
        }
    }
    // First device as weak fallback when hint empty
    if hint.is_empty() || hint == "—" {
        return Ok(lines.first().and_then(|l| parse_device_index(l)));
    }
    debug!("OpenRGB: no device matching «{product_hint}»");
    Ok(None)
}

pub async fn set_color(product_hint: &str, _zone: &str, color: RgbColor) -> Result<bool, String> {
    let Some(idx) = find_device_index(product_hint).await? else {
        return Ok(false);
    };
    // openrgb -d N -c RRGGBB  (hex without #)
    let hex = format!("{:02X}{:02X}{:02X}", color.r, color.g, color.b);
    run_openrgb(&["-d", &idx.to_string(), "-c", &hex, "-m", "static"]).await?;
    Ok(true)
}

pub async fn set_effect(
    product_hint: &str,
    effect: &str,
    color: Option<RgbColor>,
) -> Result<bool, String> {
    let Some(idx) = find_device_index(product_hint).await? else {
        return Ok(false);
    };
    let effect_lc = effect.to_ascii_lowercase();
    let mode = match effect_lc.as_str() {
        "static" => "static",
        "rainbow" | "spectrum" => "rainbow",
        "breathe" | "breathing" => "breathing",
        "wave" => "wave",
        "off" | "none" => "off",
        other => other,
    };
    let idx_s = idx.to_string();
    if let Some(c) = color {
        let hex = format!("{:02X}{:02X}{:02X}", c.r, c.g, c.b);
        match run_openrgb(&["-d", &idx_s, "-c", &hex, "-m", mode]).await {
            Ok(_) => Ok(true),
            Err(_) => {
                // Some modes ignore color
                run_openrgb(&["-d", &idx_s, "-m", mode]).await?;
                Ok(true)
            }
        }
    } else {
        run_openrgb(&["-d", &idx_s, "-m", mode]).await?;
        Ok(true)
    }
}
