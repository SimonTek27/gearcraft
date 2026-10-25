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
        let using_demo = found.is_empty();
        if using_demo {
            found = demo_devices();
        }

        // Hotplug poll runs every 15 s: rebuild addons only when the device
        // set (ids/paths) actually changed, to avoid re-initialising hardware
        // on every tick.
        let unchanged = found.len() == self.devices.len()
            && found
                .iter()
                .zip(self.devices.iter())
                .all(|(new, old)| new.id == old.id && new.path == old.path);
        if unchanged && !self.addons.is_empty() {
            return;
        }

        if using_demo {
            info!("No live devices – loading demo registry");
        }
        self.devices = found;
        self.addons.clear();
        for dev in &self.devices {
            if let Some(mut addon) = addons::steelseries::try_create(dev) {
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
            id: "ss-rival3-001".into(),
            name: "Rival 3".into(),
            vendor: "SteelSeries".into(),
            kind: DeviceKind::Mouse,
            vendor_id: 0x1038,
            product_id: 0x1824,
            path: Some("/dev/hidraw0".into()),
        },
        DeviceInfo {
            id: "ss-apex5-001".into(),
            name: "Apex 5".into(),
            vendor: "SteelSeries".into(),
            kind: DeviceKind::Keyboard,
            vendor_id: 0x1038,
            product_id: 0x1612,
            path: Some("/dev/hidraw1".into()),
        },
        DeviceInfo {
            id: "ss-arctis7-001".into(),
            name: "Arctis 7".into(),
            vendor: "SteelSeries".into(),
            kind: DeviceKind::Headset,
            vendor_id: 0x1038,
            product_id: 0x12AD,
            path: Some("/dev/hidraw2".into()),
        },
    ]
}

pub struct SharedState {
    pub registry: Arc<RwLock<DeviceRegistry>>,
    pub macros: Arc<RwLock<MacroStore>>,
    pub chatmix: Arc<RwLock<ChatMixState>>,
    pub audio_presets: Arc<RwLock<crate::audio_presets::AudioPresetStore>>,
}

pub async fn run_server(
    state: SharedState,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let path = default_socket_path();
    if path.exists() {
        let _ = std::fs::remove_file(&path);
    }
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let listener = UnixListener::bind(&path)?;
    info!("GearCraft daemon IPC listening on {}", path.display());

    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                let registry = Arc::clone(&state.registry);
                let macros = Arc::clone(&state.macros);
                let chatmix = Arc::clone(&state.chatmix);
                let audio_presets = Arc::clone(&state.audio_presets);
                tokio::spawn(async move {
                    let st = SharedState {
                        registry,
                        macros,
                        chatmix,
                        audio_presets,
                    };
                    if let Err(e) = handle_client(stream, st).await {
                        warn!("client handler error: {e}");
                    }
                });
            }
            Err(e) => error!("accept error: {e}"),
        }
    }
}

async fn handle_client(
    mut stream: UnixStream,
    state: SharedState,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    loop {
        let mut len_buf = [0u8; 4];
        match stream.read_exact(&mut len_buf).await {
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(e) => return Err(e.into()),
        }
        let len = u32::from_le_bytes(len_buf) as usize;
        if len == 0 || len > 1_048_576 {
            warn!("invalid message length: {len}");
            break;
        }
        let mut body = vec![0u8; len];
        stream.read_exact(&mut body).await?;

        let request: IpcRequest = match serde_json::from_slice(&body) {
            Ok(r) => r,
            Err(e) => {
                let err_resp = IpcResponse::failure(Uuid::nil(), format!("invalid JSON: {e}"));
                write_response(&mut stream, &err_resp).await?;
                continue;
            }
        };

        let response = dispatch(request, &state).await;
        write_response(&mut stream, &response).await?;
    }
    Ok(())
}

async fn write_response(
    stream: &mut UnixStream,
    response: &IpcResponse,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let body = serde_json::to_vec(response)?;
    let len = (body.len() as u32).to_le_bytes();
    stream.write_all(&len).await?;
    stream.write_all(&body).await?;
    stream.flush().await?;
    Ok(())
}

fn clamp_balance(b: i8) -> i8 {
    b.clamp(-100, 100)
}

fn volumes_from_balance(balance: i8) -> (u8, u8) {
    // balance +100 → game 100 chat 0; -100 → game 0 chat 100
    let b = balance as i16;
    let game = ((100 + b) / 2).clamp(0, 100) as u8;
    let chat = ((100 - b) / 2).clamp(0, 100) as u8;
    (game, chat)
}

async fn dispatch(request: IpcRequest, state: &SharedState) -> IpcResponse {
    let id = request.id;
    match request.cmd {
        IpcCommand::Ping => IpcResponse::success(id, "pong"),

        IpcCommand::GetDevices => {
            let reg = state.registry.read().await;
            IpcResponse::success(id, reg.list())
        }

        IpcCommand::GetDeviceStatus { device_id } => {
            let reg = state.registry.read().await;
            match reg.get(&device_id) {
                Some(dev) => {
                    let details = reg
                        .status_json(&device_id)
                        .unwrap_or_else(|_| serde_json::json!({}));
                    let status = DeviceStatus {
                        id: dev.id.clone(),
                        status: ConnectionStatus::Online,
                        battery: if dev.kind == DeviceKind::Headset {
                            details
                                .get("battery")
                                .and_then(|v| v.as_u64())
                                .map(|n| n as u8)
                                .or(Some(78))
                        } else {
                            None
                        },
                        details,
                    };
                    IpcResponse::success(id, status)
                }
                None => IpcResponse::failure(id, format!("device not found: {device_id}")),
            }
        }

        IpcCommand::ApplyProfile { device_id, profile } => {
            let mut reg = state.registry.write().await;
            match reg.apply_profile(&device_id, profile) {
                Ok(()) => IpcResponse::success(id, serde_json::json!({ "applied": true })),
                Err(e) => IpcResponse::failure(id, e),
            }
        }

        IpcCommand::SetColor {
            device_id,
            zone,
            color,
        } => {
            let mut reg = state.registry.write().await;
            match reg.set_color(&device_id, &zone, color) {
                Ok(()) => IpcResponse::success(id, serde_json::json!({ "set": true })),
                Err(e) => IpcResponse::failure(id, e),
            }
        }

        IpcCommand::ListAddons => IpcResponse::success(
            id,
            vec![serde_json::json!({
                "name": "steelseries",
                "version": "0.1.0",
                "protocols": ["rival3", "apex", "arctis"],
                "devices_supported": ["Rival 3", "Apex 3/5/7", "Arctis 7 / Nova 7"]
            })],
        ),

        IpcCommand::ListMacros => {
            let store = state.macros.read().await;
            IpcResponse::success(id, store.list())
        }

        IpcCommand::UpsertMacro { macro_def } => {
            let mut store = state.macros.write().await;
            store.upsert(macro_def);
            IpcResponse::success(id, serde_json::json!({ "saved": true }))
        }

        IpcCommand::DeleteMacro { macro_id } => {
            let mut store = state.macros.write().await;
            if store.delete(&macro_id) {
                IpcResponse::success(id, serde_json::json!({ "deleted": true }))
            } else {
                IpcResponse::failure(id, format!("macro not found: {macro_id}"))
            }
        }

        IpcCommand::RunMacro { macro_id } => {
            let store = state.macros.read().await;
            match store.run(&macro_id) {
                Ok(via_uinput) => IpcResponse::success(
                    id,
                    serde_json::json!({ "ran": true, "uinput": via_uinput }),
                ),
                Err(e) => IpcResponse::failure(id, e),
            }
        }

        IpcCommand::GetChatMix => {
            let cm = state.chatmix.read().await;
            let backend = crate::chatmix_pw::detect_sinks().await;
            IpcResponse::success(
                id,
                serde_json::json!({
                    "balance": cm.balance,
                    "game_volume": cm.game_volume,
                    "chat_volume": cm.chat_volume,
                    "backend": backend.backend,
                    "available": backend.available,
                    "game_sink": backend.game_sink,
                    "chat_sink": backend.chat_sink,
                    "message": backend.message,
                }),
            )
        }

        IpcCommand::SetChatMix { balance } => {
            let balance = clamp_balance(balance);
            let (game, chat) = volumes_from_balance(balance);
            let mut cm = state.chatmix.write().await;
            cm.balance = balance;
            cm.game_volume = game;
            cm.chat_volume = chat;
            info!("ChatMix balance={balance} game={game}% chat={chat}%");
            if let Err(e) = crate::config::save_chatmix(&cm) {
                warn!("ChatMix persist failed: {e}");
            }

            let apply = crate::chatmix_pw::apply_volumes(game, chat).await;
            match apply {
                Ok(backend) => IpcResponse::success(
                    id,
                    serde_json::json!({
                        "balance": cm.balance,
                        "game_volume": cm.game_volume,
                        "chat_volume": cm.chat_volume,
                        "backend": backend.backend,
                        "available": true,
                        "applied": true,
                        "message": backend.message,
                    }),
                ),
                Err(e) => {
                    // State still updated & persisted; audio not applied
                    warn!("ChatMix audio apply failed: {e}");
                    IpcResponse::success(
                        id,
                        serde_json::json!({
                            "balance": cm.balance,
                            "game_volume": cm.game_volume,
                            "chat_volume": cm.chat_volume,
                            "available": false,
                            "applied": false,
                            "message": e,
                        }),
                    )
                }
            }
        }

        IpcCommand::SetupChatMix => {
            match crate::chatmix_pw::setup_virtual_sinks().await {
                Ok(msg) => {
                    let backend = crate::chatmix_pw::detect_sinks().await;
                    IpcResponse::success(
                        id,
                        serde_json::json!({
                            "ok": true,
                            "message": msg,
                            "backend": backend.backend,
                            "available": backend.available,
                            "game_sink": backend.game_sink,
                            "chat_sink": backend.chat_sink,
                        }),
                    )
                }
                Err(e) => IpcResponse::failure(id, e),
            }
        }


        IpcCommand::ListAudioPresets => {
            let store = state.audio_presets.read().await;
            IpcResponse::success(id, store.list())
        }

        IpcCommand::LoadAudioPreset { preset_id } => {
            let preset = {
                let store = state.audio_presets.read().await;
                store.get(&preset_id).cloned()
            };
            match preset {
                Some(p) => {
                    // Update in-memory ChatMix if snapshot present
                    if let Some(ref cm) = p.chatmix {
                        let mut c = state.chatmix.write().await;
                        *c = cm.clone();
                    }
                    match crate::audio_presets::apply_preset_runtime(&p).await {
                        Ok(data) => IpcResponse::success(id, data),
                        Err(e) => IpcResponse::failure(id, e),
                    }
                }
                None => IpcResponse::failure(id, format!("preset not found: {preset_id}")),
            }
        }

        IpcCommand::SaveAudioPreset { preset } => {
            let mut store = state.audio_presets.write().await;
            match store.save(preset) {
                Ok(()) => IpcResponse::success(id, serde_json::json!({ "saved": true })),
                Err(e) => IpcResponse::failure(id, e),
            }
        }

        IpcCommand::DeleteAudioPreset { preset_id } => {
            let mut store = state.audio_presets.write().await;
            match store.delete(&preset_id) {
                Ok(true) => IpcResponse::success(id, serde_json::json!({ "deleted": true })),
                Ok(false) => IpcResponse::failure(id, format!("preset not found: {preset_id}")),
                Err(e) => IpcResponse::failure(id, e),
            }
        }

        IpcCommand::ListPresetRules => {
            let store = state.audio_presets.read().await;
            IpcResponse::success(id, store.list_rules())
        }

        IpcCommand::SetPresetRule { rule } => {
            let mut store = state.audio_presets.write().await;
            match store.set_rule(rule) {
                Ok(()) => IpcResponse::success(id, serde_json::json!({ "ok": true })),
                Err(e) => IpcResponse::failure(id, e),
            }
        }

        IpcCommand::DeletePresetRule {
            device_id,
            route_id,
        } => {
            let mut store = state.audio_presets.write().await;
            match store.delete_rule(&device_id, &route_id) {
                Ok(true) => IpcResponse::success(id, serde_json::json!({ "deleted": true })),
                Ok(false) => IpcResponse::failure(id, "rule not found"),
                Err(e) => IpcResponse::failure(id, e),
            }
        }

        IpcCommand::Shutdown => {
            info!("shutdown requested via IPC");
            IpcResponse::success(id, "shutting down")
        }
    }
}

#[allow(dead_code)]
pub async fn send_request(
    path: &Path,
    cmd: IpcCommand,
) -> Result<IpcResponse, Box<dyn std::error::Error + Send + Sync>> {
    let mut stream = UnixStream::connect(path).await?;
    let request = IpcRequest {
        id: Uuid::new_v4(),
        cmd,
    };
    let body = serde_json::to_vec(&request)?;
    let len = (body.len() as u32).to_le_bytes();
    stream.write_all(&len).await?;
    stream.write_all(&body).await?;
    stream.flush().await?;

    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf).await?;
    let resp_len = u32::from_le_bytes(len_buf) as usize;
    let mut resp_body = vec![0u8; resp_len];
    stream.read_exact(&mut resp_body).await?;
    Ok(serde_json::from_slice(&resp_body)?)
}
