//! libratbag / Piper stack via `ratbagctl` CLI.
//!
//! Provides DPI read/write when ratbagd is running.

use std::process::Stdio;

use tokio::process::Command;
use tracing::info;

pub async fn available() -> bool {
    Command::new("ratbagctl")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await
        .map(|s| s.success())
        .unwrap_or(false)
}

async fn run(args: &[&str]) -> Result<String, String> {
    let out = Command::new("ratbagctl")
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|e| format!("ratbagctl: {e}"))?;
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    if !out.status.success() {
        return Err(format!("ratbagctl failed: {stderr} {stdout}"));
    }
    Ok(stdout)
}

/// List ratbag device names (first column / device id strings).
pub async fn list_devices() -> Result<Vec<String>, String> {
    let out = run(&[]).await?;
    // ratbagctl with no args lists devices
    let mut names = Vec::new();
    for line in out.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        // formats: "name:    Device Name" or just device index lines
        if let Some(rest) = line.strip_prefix("name:") {
            names.push(rest.trim().to_string());
        } else if !line.contains(':') {
            names.push(line.to_string());
        }
    }
    // Fallback: `ratbagctl list` on newer versions
    if names.is_empty() {
        if let Ok(out) = run(&["list"]).await {
            for line in out.lines() {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                // "0: Logitech G Pro"
                if let Some((_, name)) = line.split_once(':') {
                    names.push(name.trim().to_string());
                } else {
                    names.push(line.to_string());
                }
            }
        }
    }
    Ok(names)
}

fn match_device(list_line: &str, hint: &str) -> bool {
    let a = list_line.to_ascii_lowercase();
    let b = hint.to_ascii_lowercase();
    a.contains(&b) || b.contains(&a)
}

pub async fn resolve_ratbag_name(product_hint: &str) -> Result<String, String> {
    let list = list_devices().await?;
    if list.is_empty() {
        return Err("ratbagctl: no devices (is ratbagd running? install piper)".into());
    }
    for n in &list {
        if match_device(n, product_hint) {
            return Ok(n.clone());
        }
    }
    // Try using index 0 as device specifier
    if let Ok(out) = run(&["list"]).await {
        for line in out.lines() {
            if let Some((idx, name)) = line.split_once(':') {
                if match_device(name, product_hint) {
                    return Ok(idx.trim().to_string());
                }
            }
        }
        // first index
        if let Some(line) = out.lines().next() {
            if let Some((idx, _)) = line.split_once(':') {
                return Ok(idx.trim().to_string());
            }
        }
    }
    Ok(list[0].clone())
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct MouseInfo {
    pub ratbag_name: String,
    pub dpi: Option<u32>,
    pub dpi_min: Option<u32>,
    pub dpi_max: Option<u32>,
    pub profiles: u32,
    pub active_profile: u32,
    pub backend: &'static str,
}

pub async fn get_mouse_info(product_hint: &str) -> Result<MouseInfo, String> {
    let name = resolve_ratbag_name(product_hint).await?;
    let mut info = MouseInfo {
        ratbag_name: name.clone(),
        dpi: None,
        dpi_min: None,
        dpi_max: None,
        profiles: 1,
        active_profile: 0,
        backend: "ratbagctl",
    };
    // ratbagctl <device> dpi get
    if let Ok(out) = run(&[&name, "dpi", "get"]).await {
        // "dpi: 800" or just number
        for tok in out.split_whitespace() {
            if let Ok(v) = tok.trim_matches(|c: char| !c.is_ascii_digit()).parse() {
                info.dpi = Some(v);
                break;
            }
        }
        if info.dpi.is_none() {
            if let Ok(v) = out.trim().parse() {
                info.dpi = Some(v);
            }
        }
    }
    if let Ok(out) = run(&[&name, "dpi", "get-default"]).await {
        let _ = out;
    }
    // resolution range — best effort from `ratbagctl <dev> info`
    if let Ok(out) = run(&[&name, "info"]).await {
        for line in out.lines() {
            let l = line.to_ascii_lowercase();
            if l.contains("dpi") {
                // scrape numbers
                let nums: Vec<u32> = line
                    .split(|c: char| !c.is_ascii_digit())
                    .filter_map(|s| s.parse().ok())
                    .filter(|&n| n >= 50)
                    .collect();
                if nums.len() >= 2 {
                    info.dpi_min = Some(nums[0]);
                    info.dpi_max = Some(*nums.last().unwrap());
                }
                if info.dpi.is_none() && !nums.is_empty() {
                    info.dpi = Some(nums[0]);
                }
            }
            if l.contains("profile") && l.contains("count") {
                if let Some(n) = line
                    .split_whitespace()
                    .rev()
                    .find_map(|t| t.parse().ok())
                {
                    info.profiles = n;
                }
            }
        }
    }
    Ok(info)
}

pub async fn set_dpi(product_hint: &str, dpi: u32, profile: u32) -> Result<String, String> {
    let name = resolve_ratbag_name(product_hint).await?;
    // Switch profile if needed
    if profile > 0 {
        let _ = run(&[&name, "profile", "active", "set", &profile.to_string()]).await;
    }
    run(&[&name, "dpi", "set", &dpi.to_string()]).await?;
    // Commit if supported
    let _ = run(&[&name, "commit"]).await;
    let msg = format!("DPI {dpi} set on «{name}» via ratbagctl");
    info!("{msg}");
    Ok(msg)
}
