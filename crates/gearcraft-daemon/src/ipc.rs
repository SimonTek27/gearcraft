//! Unix domain socket IPC server.
//!
//! Protocol: length-prefixed JSON (u32 LE length + UTF-8 JSON body).

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use gearcraft_core::{
    default_socket_path, ChatMixState, ConnectionStatus, DeviceInfo, DeviceKind, DeviceStatus,
    IpcCommand, IpcRequest, IpcResponse, PeripheralAddon, RgbColor,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::RwLock;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::addons;
use crate::macros_store::MacroStore;
use crate::usb;

/// Runtime state: discovered devices + loaded addons.
pub struct DeviceRegistry {
    devices: Vec<DeviceInfo>,
    addons: HashMap<String, Box<dyn PeripheralAddon>>,
}

impl DeviceRegistry {
    pub fn new() -> Self {
        Self {
            devices: Vec::new(),
            addons: HashMap::new(),
        }
    }

    pub fn refresh(&mut self) {
        let mut found = usb::scan_devices();
        if found.is_empty() {
            info!("No live devices – loading demo registry");
            found = demo_devices();
        }
        self.devices = found;
        self.addons.clear();
        for dev in &self.devices {
            if let Some(mut addon) = addons::steelseries::try_create(dev) {
                if let Err(e) = addon.initialize() {
                    warn!("addon init failed for {}: {e}", dev.id);
                }
                self.addons.insert(dev.id.clone(), addon);
            } else if let Some(mut addon) = addons::razer::try_create(dev) {
                if let Err(e) = addon.initialize() {
                    warn!("addon init failed for {}: {e}", dev.id);
                }
                self.addons.insert(dev.id.clone(), addon);
            }
        }
        info!(
            "Registry: {} device(s), {} addon(s)",
            self.devices.len(),
            self.addons.len()
        );
    }

    pub fn list(&self) -> &[DeviceInfo] {
        &self.devices
    }

    pub fn get(&self, id: &str) -> Option<&DeviceInfo> {
        self.devices.iter().find(|d| d.id == id)
    }

    pub fn set_color(
        &mut self,
        device_id: &str,
        zone: &str,
        color: RgbColor,
    ) -> Result<(), String> {
        let addon = self
            .addons
            .get_mut(device_id)
            .ok_or_else(|| format!("no addon for device: {device_id}"))?;
        addon.set_color(zone, color).map_err(|e| e.to_string())
    }

    pub fn apply_profile(
        &mut self,
        device_id: &str,
        profile: serde_json::Value,
    ) -> Result<(), String> {
        let addon = self
            .addons
            .get_mut(device_id)
            .ok_or_else(|| format!("no addon for device: {device_id}"))?;
        addon.apply_profile(profile).map_err(|e| e.to_string())
    }

    pub fn status_json(&self, device_id: &str) -> Result<serde_json::Value, String> {
        let addon = self
            .addons
            .get(device_id)
            .ok_or_else(|| format!("no addon for device: {device_id}"))?;
        addon.get_status().map_err(|e| e.to_string())
    }
}

fn demo_devices() -> Vec<DeviceInfo> {
    vec![
        DeviceInfo {
            id: "demo-rival3".into(),
            name: "Rival 3 (demo)".into(),
            vendor: "SteelSeries".into(),
            kind: DeviceKind::Mouse,
            vendor_id: 0x1038,
            product_id: 0x1824,
            path: None,
        },
        DeviceInfo {
            id: "demo-apex".into(),
            name: "Apex 3 (demo)".into(),
            vendor: "SteelSeries".into(),
            kind: DeviceKind::Keyboard,
            vendor_id: 0x1038,
            product_id: 0x161a,
            path: None,
        },
        DeviceInfo {
            id: "demo-arctis".into(),
            name: "Arctis 7 (demo)".into(),
            vendor: "SteelSeries".into(),
            kind: DeviceKind::Headset,
            vendor_id: 0x1038,
            product_id: 0x12ad,
            path: None,
        },
    ]
}

pub struct SharedState {
    pub registry: Arc<RwLock<DeviceRegistry>>,
    pub macros: Arc<RwLock<MacroStore>>,
    pub chatmix: Arc<RwLock<ChatMixState>>,
    pub audio_presets: Arc<RwLock<crate::audio_presets::AudioPresetStore>>,
}

pub async fn run_server(state: SharedState) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let path = default_socket_path();
    if Path::new(&path).exists() {
        let _ = std::fs::remove_file(&path);
    }
    if let Some(parent) = Path::new(&path).parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let listener = UnixListener::bind(&path)?;
    info!("IPC listening on {path}");

    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                let st = SharedState {
                    registry: Arc::clone(&state.registry),
                    macros: Arc::clone(&state.macros),
                    chatmix: Arc::clone(&state.chatmix),
                    audio_presets: Arc::clone(&state.audio_presets),
                };
                tokio::spawn(async move {
                    if let Err(e) = handle_client(stream, st).await {
                        error!("client error: {e}");
                    }
                });
            }
            Err(e) => error!("accept: {e}"),
        }
    }
}

async fn handle_client(
    mut stream: UnixStream,
    state: SharedState,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    loop {
        let mut len_buf = [0u8; 4];
        if stream.read_exact(&mut len_buf).await.is_err() {
            break;
        }
        let len = u32::from_le_bytes(len_buf) as usize;
        if len > 16 * 1024 * 1024 {
            return Err("frame too large".into());
        }
        let mut body = vec![0u8; len];
        stream.read_exact(&mut body).await?;
        let req: IpcRequest = serde_json::from_slice(&body)?;
        let resp = dispatch(req, &state).await;
        let out = serde_json::to_vec(&resp)?;
        let len_out = (out.len() as u32).to_le_bytes();
        stream.write_all(&len_out).await?;
        stream.write_all(&out).await?;
    }
    Ok(())
}

async fn dispatch(req: IpcRequest, state: &SharedState) -> IpcResponse {
    let id = req.id.clone();
    match handle_cmd(req.command, state).await {
        Ok(data) => IpcResponse {
            id,
            ok: true,
            data,
            error: None,
        },
        Err(e) => IpcResponse {
            id,
            ok: false,
            data: None,
            error: Some(e),
        },
    }
}

async fn handle_cmd(
    cmd: IpcCommand,
    state: &SharedState,
) -> Result<Option<serde_json::Value>, String> {
    match cmd {
        IpcCommand::Ping => Ok(Some(serde_json::json!({"pong": true}))),
        IpcCommand::ListDevices => {
            let reg = state.registry.read().await;
            Ok(Some(serde_json::to_value(reg.list()).map_err(|e| e.to_string())?))
        }
        IpcCommand::GetDevice { device_id } => {
            let reg = state.registry.read().await;
            let d = reg
                .get(&device_id)
                .ok_or_else(|| format!("unknown device: {device_id}"))?;
            Ok(Some(serde_json::to_value(d).map_err(|e| e.to_string())?))
        }
        IpcCommand::RefreshDevices => {
            let mut reg = state.registry.write().await;
            reg.refresh();
            Ok(Some(serde_json::json!({"count": reg.list().len()})))
        }
        IpcCommand::SetColor {
            device_id,
            zone,
            color,
        } => {
            let mut reg = state.registry.write().await;
            reg.set_color(&device_id, &zone, color)?;
            Ok(Some(serde_json::json!({"applied": true})))
        }
        IpcCommand::ApplyProfile {
            device_id,
            profile,
        } => {
            let mut reg = state.registry.write().await;
            reg.apply_profile(&device_id, profile)?;
            Ok(Some(serde_json::json!({"applied": true})))
        }
        IpcCommand::GetStatus { device_id } => {
            let reg = state.registry.read().await;
            let st = reg.status_json(&device_id)?;
            Ok(Some(st))
        }
        IpcCommand::ListMacros => {
            let macros = state.macros.read().await;
            Ok(Some(
                serde_json::to_value(macros.list()).map_err(|e| e.to_string())?,
            ))
        }
        IpcCommand::UpsertMacro { macro_def } => {
            let mut macros = state.macros.write().await;
            macros.upsert(macro_def)?;
            Ok(Some(serde_json::json!({"saved": true})))
        }
        IpcCommand::DeleteMacro { macro_id } => {
            let mut macros = state.macros.write().await;
            macros.delete(&macro_id)?;
            Ok(Some(serde_json::json!({"deleted": true})))
        }
        IpcCommand::RunMacro { macro_id } => {
            let macros = state.macros.read().await;
            let def = macros
                .get(&macro_id)
                .ok_or_else(|| format!("unknown macro: {macro_id}"))?
                .clone();
            drop(macros);
            crate::uinput_macros::run_macro(&def).await?;
            Ok(Some(serde_json::json!({"ran": true})))
        }
        IpcCommand::GetChatMix => {
            let cm = state.chatmix.read().await;
            Ok(Some(serde_json::to_value(&*cm).map_err(|e| e.to_string())?))
        }
        IpcCommand::SetChatMix { balance } => {
            let mut cm = state.chatmix.write().await;
            cm.balance = balance.clamp(-100, 100);
            // map balance to volumes
            if cm.balance >= 0 {
                cm.game_volume = 100;
                cm.chat_volume = (100 - cm.balance) as u8;
            } else {
                cm.chat_volume = 100;
                cm.game_volume = (100 + cm.balance) as u8;
            }
            let g = cm.game_volume;
            let c = cm.chat_volume;
            drop(cm);
            let info = crate::chatmix_pw::apply_volumes(g, c).await?;
            Ok(Some(serde_json::json!({
                "balance": balance,
                "backend": info.backend,
                "message": info.message,
            })))
        }
        IpcCommand::SetupChatMix => {
            let msg = crate::chatmix_pw::setup_virtual_sinks().await?;
            Ok(Some(serde_json::json!({"message": msg})))
        }
        IpcCommand::ListAudioPresets => {
            let store = state.audio_presets.read().await;
            Ok(Some(
                serde_json::to_value(store.list()).map_err(|e| e.to_string())?,
            ))
        }
        IpcCommand::LoadAudioPreset { preset_id } => {
            let preset = {
                let store = state.audio_presets.read().await;
                store
                    .get(&preset_id)
                    .cloned()
                    .ok_or_else(|| format!("unknown preset: {preset_id}"))?
            };
            let result = crate::audio_presets::apply_preset_runtime(&preset).await?;
            if let Some(cm) = &preset.chatmix {
                let mut state_cm = state.chatmix.write().await;
                *state_cm = cm.clone();
            }
            Ok(Some(result))
        }
        IpcCommand::SaveAudioPreset { preset } => {
            let mut store = state.audio_presets.write().await;
            store.save(preset)?;
            Ok(Some(serde_json::json!({"saved": true})))
        }
        IpcCommand::DeleteAudioPreset { preset_id } => {
            let mut store = state.audio_presets.write().await;
            store.delete(&preset_id)?;
            Ok(Some(serde_json::json!({"deleted": true})))
        }
        IpcCommand::ListPresetRules => {
            let store = state.audio_presets.read().await;
            Ok(Some(
                serde_json::to_value(store.list_rules()).map_err(|e| e.to_string())?,
            ))
        }
        IpcCommand::SetPresetRule { rule } => {
            let mut store = state.audio_presets.write().await;
            store.set_rule(rule)?;
            Ok(Some(serde_json::json!({"saved": true})))
        }
        IpcCommand::DeletePresetRule {
            device_id,
            route_id,
        } => {
            let mut store = state.audio_presets.write().await;
            let rid = route_id.unwrap_or_default();
            store.delete_rule(&device_id, &rid)?;
            Ok(Some(serde_json::json!({"deleted": true})))
        }
        IpcCommand::GetConnectionStatus => Ok(Some(serde_json::json!({
            "status": "online",
            "socket": default_socket_path(),
        }))),
        _ => Err(format!("unhandled command")),
    }
}
