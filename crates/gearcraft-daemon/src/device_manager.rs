//! Sonar Device Manager — physical output per channel, priority & backup.

use std::collections::HashMap;
use std::process::Stdio;

use gearcraft_core::{
    AudioOutputDevice, BackupMode, ChannelDevicePolicy, DeviceManagerState, SonarChannelId,
};
use tokio::process::Command;
use tracing::{info, warn};

use crate::config;

pub struct DeviceManager {
    pub state: DeviceManagerState,
    loopbacks: HashMap<String, u32>,
}

impl DeviceManager {
    pub fn load() -> Self {
        let state = config::load_device_manager().unwrap_or_default();
        info!(
            "device manager: link_all={} backup={:?} channels={}",
            state.link_all,
            state.backup_mode,
            state.channels.len()
        );
        Self {
            state,
            loopbacks: HashMap::new(),
        }
    }

    pub fn save(&self) -> Result<(), String> {
        config::save_device_manager(&self.state)
    }

    pub fn set_state(&mut self, state: DeviceManagerState) {
        self.state = state;
    }

    pub fn set_policy(&mut self, mut policy: ChannelDevicePolicy) {
        policy.sync_priority_from_primary_backup();
        if let Some(slot) = self
            .state
            .channels
            .iter_mut()
            .find(|c| c.channel == policy.channel)
        {
            *slot = policy;
        } else {
            self.state.channels.push(policy);
        }
    }

    pub fn link_all(&mut self, device_id: Option<String>, enabled: bool) {
        self.state.link_all = enabled;
        if enabled {
            if let Some(id) = device_id {
                for ch in &mut self.state.channels {
                    ch.priority.retain(|x| x != &id);
                    ch.priority.insert(0, id.clone());
                    ch.excluded.retain(|x| x != &id);
                }
            }
        }
    }
}

pub async fn list_outputs() -> Result<Vec<AudioOutputDevice>, String> {
    let out = Command::new("pactl")
        .args(["list", "sinks"])
        .output()
        .await
        .map_err(|e| format!("pactl: {e}"))?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).into_owned());
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let mut devices = Vec::new();
    let mut name = String::new();
    let mut description = String::new();
    let mut state_line = String::new();

    let flush = |devices: &mut Vec<AudioOutputDevice>,
                 name: &mut String,
                 description: &mut String,
                 state_line: &mut String| {
        if name.is_empty() {
            return;
        }
        let is_virtual = name.starts_with("gearcraft_")
            || description.to_ascii_lowercase().contains("gearcraft");
        let available = !state_line.to_ascii_lowercase().contains("invalid");
        devices.push(AudioOutputDevice {
            id: name.clone(),
            name: name.clone(),
            description: if description.is_empty() {
                name.clone()
            } else {
                description.clone()
            },
            is_virtual,
            available: available || is_virtual,
        });
        name.clear();
        description.clear();
        state_line.clear();
    };

    for line in text.lines() {
        let t = line.trim();
        if t.starts_with("Sink #") {
            flush(&mut devices, &mut name, &mut description, &mut state_line);
        } else if let Some(v) = t.strip_prefix("Name:") {
            name = v.trim().to_string();
        } else if let Some(v) = t.strip_prefix("Description:") {
            description = v.trim().to_string();
        } else if let Some(v) = t.strip_prefix("State:") {
            state_line = v.trim().to_string();
        }
    }
    flush(&mut devices, &mut name, &mut description, &mut state_line);
    Ok(devices)
}

fn pick_device(
    policy: &ChannelDevicePolicy,
    devices: &[AudioOutputDevice],
) -> Option<String> {
    let available: HashMap<&str, &AudioOutputDevice> = devices
        .iter()
        .filter(|d| d.available && !d.is_virtual)
        .map(|d| (d.id.as_str(), d))
        .collect();

    for id in &policy.priority {
        if policy.excluded.iter().any(|e| e == id) {
            continue;
        }
        if available.contains_key(id.as_str()) {
            return Some(id.clone());
        }
    }
    for d in devices {
        if d.is_virtual || !d.available {
            continue;
        }
        if policy.excluded.iter().any(|e| e == &d.id) {
            continue;
        }
        return Some(d.id.clone());
    }
    None
}

async fn unload_module(id: u32) {
    let _ = Command::new("pactl")
        .args(["unload-module", &id.to_string()])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await;
}

async fn load_loopback(virtual_sink: &str, physical_sink: &str) -> Result<u32, String> {
    let source = format!("{virtual_sink}.monitor");
    let out = Command::new("pactl")
        .args([
            "load-module",
            "module-loopback",
            &format!("source={source}"),
            &format!("sink={physical_sink}"),
            "latency_msec=20",
            "sink_dont_move=true",
            "source_dont_move=true",
        ])
        .output()
        .await
        .map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).into_owned());
    }
    let id_str = String::from_utf8_lossy(&out.stdout).trim().to_string();
    id_str
        .parse::<u32>()
        .map_err(|_| format!("bad module id: {id_str}"))
}

pub async fn apply(mgr: &mut DeviceManager) -> Result<serde_json::Value, String> {
    let _ = crate::chatmix_pw::setup_virtual_sinks().await;
    let devices = list_outputs().await.unwrap_or_default();
    let mut applied = Vec::new();

    if mgr.state.link_all {
        let primary = mgr
            .state
            .channels
            .iter()
            .find(|c| c.channel == SonarChannelId::Game)
            .and_then(|c| c.priority.first().cloned())
            .or_else(|| {
                devices
                    .iter()
                    .find(|d| !d.is_virtual && d.available)
                    .map(|d| d.id.clone())
            });
        if let Some(ref id) = primary {
            for ch in &mut mgr.state.channels {
                ch.priority.retain(|x| x != id);
                ch.priority.insert(0, id.clone());
            }
        }
    }

    for ch in SonarChannelId::all() {
        let policy = mgr
            .state
            .channels
            .iter()
            .find(|c| c.channel == ch)
            .cloned()
            .unwrap_or_else(|| ChannelDevicePolicy::new(ch));

        let chosen = pick_device(&policy, &devices);
        let key = ch.as_str().to_string();
        if let Some(mid) = mgr.loopbacks.remove(&key) {
            unload_module(mid).await;
        }

        if let Some(ref phys) = chosen {
            match load_loopback(ch.virtual_sink(), phys).await {
                Ok(mid) => {
                    mgr.loopbacks.insert(key, mid);
                    info!("device manager: {} → {} (module {mid})", ch.as_str(), phys);
                    applied.push(serde_json::json!({
                        "channel": ch.as_str(),
                        "device": phys,
                        "module": mid,
                    }));
                }
                Err(e) => {
                    warn!("loopback {} → {}: {e}", ch.as_str(), phys);
                    applied.push(serde_json::json!({
                        "channel": ch.as_str(),
                        "device": phys,
                        "error": e,
                    }));
                }
            }
        } else {
            applied.push(serde_json::json!({
                "channel": ch.as_str(),
                "device": null,
            }));
        }

        if let Some(slot) = mgr.state.channels.iter_mut().find(|c| c.channel == ch) {
            slot.active_device_id = chosen.clone();
        }
    }

    let _ = mgr.save();
    Ok(serde_json::json!({
        "applied": applied,
        "link_all": mgr.state.link_all,
        "backup_mode": mgr.state.backup_mode,
    }))
}

pub async fn failover_tick(mgr: &mut DeviceManager) {
    if mgr.state.backup_mode != BackupMode::Automatic {
        return;
    }
    let devices = match list_outputs().await {
        Ok(d) => d,
        Err(_) => return,
    };
    let mut need = false;
    for ch in &mgr.state.channels {
        if let Some(ref active) = ch.active_device_id {
            let still_ok = devices.iter().any(|d| &d.id == active && d.available);
            if !still_ok {
                need = true;
                break;
            }
        }
    }
    if need {
        info!("device manager: primary missing – automatic failover");
        let _ = apply(mgr).await;
    }
}

pub async fn snapshot(mgr: &DeviceManager) -> serde_json::Value {
    let devices = list_outputs().await.unwrap_or_default();
    serde_json::json!({
        "state": mgr.state,
        "devices": devices,
    })
}
