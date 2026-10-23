//! Unix domain socket IPC server.
//!
//! Protocol: length-prefixed JSON (u32 LE length + UTF-8 JSON body).

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use gearcraft_core::{
    default_socket_path, ChatMixState, ConnectionStatus, DeviceInfo, DeviceKind, DeviceStatus,
    IpcCommand, IpcRequest, IpcResponse, MacroDef, PeripheralAddon, RgbColor,
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

    pub fn status(&self, device_id: &str) -> DeviceStatus {
        let online = self.addons.contains_key(device_id);
        DeviceStatus {
            id: device_id.to_string(),
            status: if online {
                ConnectionStatus::Online
            } else {
                ConnectionStatus::Offline
            },
            battery: self.addons.get(device_id).and_then(|a| a.battery()),
            details: Default::default(),
        }
    }
}

fn demo_devices() -> Vec<DeviceInfo> {
    addons::steelseries::demo_devices()
}

pub struct SharedState {
    pub registry: Arc<RwLock<DeviceRegistry>>,
    pub macros: Arc<RwLock<MacroStore>>,
    pub chatmix: Arc<RwLock<ChatMixState>>,
}

pub async fn run_server(state: SharedState) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let path = default_socket_path();
    if path.exists() {
        let _ = std::fs::remove_file(&path);
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let listener = UnixListener::bind(&path)?;
    info!("IPC listening on {}", path.display());

    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                let st = SharedState {
                    registry: Arc::clone(&state.registry),
                    macros: Arc::clone(&state.macros),
                    chatmix: Arc::clone(&state.chatmix),
                };
                tokio::spawn(async move {
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
            let status = reg.status(&device_id);
            IpcResponse::success(id, status)
        }

        IpcCommand::ApplyProfile { device_id, profile } => {
            let mut reg = state.registry.write().await;
            match reg.apply_profile(&device_id, profile) {
                Ok(()) => IpcResponse::success(id, "ok"),
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
                Ok(()) => IpcResponse::success(id, "ok"),
                Err(e) => IpcResponse::failure(id, e),
            }
        }

        IpcCommand::ListAddons => IpcResponse::success(
            id,
            serde_json::json!([{"name": "steelseries", "vendor_id": 0x1038}]),
        ),

        IpcCommand::ListMacros => {
            let store = state.macros.read().await;
            IpcResponse::success(id, store.list())
        }

        IpcCommand::UpsertMacro { macro_def } => {
            let mut store = state.macros.write().await;
            store.upsert(macro_def);
            IpcResponse::success(id, "ok")
        }

        IpcCommand::DeleteMacro { macro_id } => {
            let mut store = state.macros.write().await;
            if store.delete(&macro_id) {
                IpcResponse::success(id, "ok")
            } else {
                IpcResponse::failure(id, "macro not found")
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
            IpcResponse::success(id, cm.clone())
        }

        IpcCommand::SetChatMix { balance } => {
            let balance = clamp_balance(balance);
            let (game, chat) = volumes_from_balance(balance);
            match crate::chatmix_pw::apply_volumes(game, chat).await {
                Ok(info) => {
                    let mut cm = state.chatmix.write().await;
                    cm.balance = balance as f32;
                    cm.game_volume = game as f32 / 100.0;
                    cm.chat_volume = chat as f32 / 100.0;
                    cm.enabled = info.available;
                    IpcResponse::success(
                        id,
                        serde_json::json!({
                            "balance": balance,
                            "game_volume": game,
                            "chat_volume": chat,
                            "backend": info.backend,
                            "available": info.available,
                            "applied": true,
                            "message": info.message,
                        }),
                    )
                }
                Err(e) => {
                    let mut cm = state.chatmix.write().await;
                    cm.balance = balance as f32;
                    IpcResponse::success(
                        id,
                        serde_json::json!({
                            "balance": balance,
                            "game_volume": game,
                            "chat_volume": chat,
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
                    let info = crate::chatmix_pw::detect_sinks().await;
                    IpcResponse::success(
                        id,
                        serde_json::json!({
                            "ok": true,
                            "message": msg,
                            "backend": info.backend,
                            "available": info.available,
                        }),
                    )
                }
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
