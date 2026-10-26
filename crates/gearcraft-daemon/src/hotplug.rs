//! USB hotplug watcher — triggers registry refresh when devices appear/leave.
//!
//! Prefers `udevadm monitor --udev --subsystem-match=usb`. Falls back to a
//! short polling interval if udevadm is unavailable.

use std::process::Stdio;
use std::sync::Arc;

use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::audio_presets::{self, AudioPresetStore};
use crate::ipc::DeviceRegistry;

/// Watch for USB changes and refresh the registry; auto-apply audio presets
/// when a matching headset/device connects.
pub async fn run(
    registry: Arc<RwLock<DeviceRegistry>>,
    presets: Arc<RwLock<AudioPresetStore>>,
) {
    let mut known: std::collections::HashSet<String> = {
        let reg = registry.read().await;
        reg.list().iter().map(|d| d.id.clone()).collect()
    };

    if which_udevadm().await {
        info!("hotplug: udevadm monitor active");
        loop {
            match run_udevadm_session(&registry, &presets, &mut known).await {
                Ok(()) => warn!("hotplug: udevadm exited, restarting in 3s"),
                Err(e) => warn!("hotplug: udevadm error: {e}; retry in 5s"),
            }
            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
        }
    } else {
        info!("hotplug: udevadm missing – polling every 5s");
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(5));
        loop {
            interval.tick().await;
            refresh_and_apply(&registry, &presets, &mut known).await;
        }
    }
}

async fn which_udevadm() -> bool {
    Command::new("udevadm")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await
        .map(|s| s.success())
        .unwrap_or(false)
}

async fn run_udevadm_session(
    registry: &Arc<RwLock<DeviceRegistry>>,
    presets: &Arc<RwLock<AudioPresetStore>>,
    known: &mut std::collections::HashSet<String>,
) -> Result<(), String> {
    let mut child = Command::new("udevadm")
        .args(["monitor", "--udev", "--subsystem-match=usb", "--property"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| format!("spawn udevadm: {e}"))?;

    let stdout = child.stdout.take().ok_or("no stdout")?;
    let mut reader = BufReader::new(stdout).lines();
    let mut pending = false;
    let mut debounce = tokio::time::interval(std::time::Duration::from_millis(400));
    debounce.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            line = reader.next_line() => {
                match line {
                    Ok(Some(l)) => {
                        let lower = l.to_ascii_lowercase();
                        if lower.contains("add")
                            || lower.contains("remove")
                            || lower.contains("bind")
                            || lower.contains("unbind")
                        {
                            pending = true;
                            debug!("hotplug event: {l}");
                        }
                    }
                    Ok(None) => break,
                    Err(e) => return Err(e.to_string()),
                }
            }
            _ = debounce.tick() => {
                if pending {
                    pending = false;
                    refresh_and_apply(registry, presets, known).await;
                }
            }
        }
    }
    let _ = child.wait().await;
    Ok(())
}

async fn refresh_and_apply(
    registry: &Arc<RwLock<DeviceRegistry>>,
    presets: &Arc<RwLock<AudioPresetStore>>,
    known: &mut std::collections::HashSet<String>,
) {
    let (appeared, all_ids) = {
        let mut reg = registry.write().await;
        reg.refresh();
        let all: std::collections::HashSet<String> =
            reg.list().iter().map(|d| d.id.clone()).collect();
        let appeared: Vec<(String, String)> = reg
            .list()
            .iter()
            .filter(|d| !known.contains(&d.id))
            .map(|d| (d.id.clone(), d.name.clone()))
            .collect();
        (appeared, all)
    };

    known.retain(|id| all_ids.contains(id));

    for (id, name) in appeared {
        known.insert(id.clone());
        info!("device connected: {name} ({id})");
        let preset = {
            let store = presets.read().await;
            if let Some(p) = store.rule_for(&id, "").cloned() {
                Some(p)
            } else {
                let mut found = None;
                for r in store.list_rules() {
                    let match_id = id.contains(&r.device_id)
                        || name
                            .to_ascii_lowercase()
                            .contains(&r.device_id.to_ascii_lowercase());
                    if match_id {
                        if let Some(p) = store.get(&r.preset_id).cloned() {
                            found = Some(p);
                            break;
                        }
                    }
                }
                found
            }
        };
        if let Some(p) = preset {
            info!("auto-apply preset «{}» for device {name}", p.name);
            match audio_presets::apply_preset_runtime(&p).await {
                Ok(_) => info!("preset «{}» applied on connect", p.name),
                Err(e) => warn!("preset apply on connect failed: {e}"),
            }
        }
    }
}
