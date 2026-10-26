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
        // Demo devices only when explicitly requested (avoids masking missing udev/permissions)
        if found.is_empty() {
            if std::env::var("GEARCRAFT_DEMO").ok().as_deref() == Some("1") {
                info!("No live devices – loading demo registry (GEARCRAFT_DEMO=1)");
                found = demo_devices();
            } else {
                info!("No live devices – empty registry (set GEARCRAFT_DEMO=1 for sample gear)");
            }
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

    pub fn device_vendor_name(&self, device_id: &str) -> (String, String) {
        if let Some(d) = self.devices.iter().find(|d| d.id == device_id) {
            return (d.vendor.clone(), d.name.clone());
        }
        if let Some(a) = self.addons.get(device_id) {
            let info = a.device_info();
            return (info.vendor, info.name);
        }
        (String::new(), device_id.to_string())
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

    pub fn set_effect(
        &mut self,
        device_id: &str,
        zone: &str,
        effect: &str,
        color: Option<RgbColor>,
    ) -> Result<(), String> {
        let addon = self
            .addons
            .get_mut(device_id)
            .ok_or_else(|| format!("no addon for device: {device_id}"))?;
        addon
            .set_effect(zone, effect, color)
            .map_err(|e| e.to_string())
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
    pub device_manager: Arc<RwLock<crate::device_manager::DeviceManager>>,
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
    // Make the socket reachable by members of the plugdev group (udev flow).
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o660));
        if let Some(gid) = plugdev_gid() {
            if let Some(p) = path.to_str() {
                if let Ok(cpath) = std::ffi::CString::new(p) {
                    unsafe {
                        libc::chown(cpath.as_ptr(), u32::MAX, gid);
                    }
                }
            }
        }
    }
    info!("GearCraft daemon IPC listening on {}", path.display());

    let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
    let mut sigint = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())?;

    loop {
        tokio::select! {
            res = listener.accept() => {
                match res {
                    Ok((stream, _)) => {
                        let registry = Arc::clone(&state.registry);
                        let macros = Arc::clone(&state.macros);
                        let chatmix = Arc::clone(&state.chatmix);
                        let audio_presets = Arc::clone(&state.audio_presets);
                        let device_manager = Arc::clone(&state.device_manager);
                        tokio::spawn(async move {
                            let st = SharedState {
                                registry,
                                macros,
                                chatmix,
                                audio_presets,
                                device_manager,
                            };
                            if let Err(e) = handle_client(stream, st).await {
                                warn!("client handler error: {e}");
                            }
                        });
                    }
                    Err(e) => error!("accept error: {e}"),
                }
            }
            _ = sigterm.recv() => {
                info!("SIGTERM received — shutting down");
                break;
            }
            _ = sigint.recv() => {
                info!("SIGINT received — shutting down");
                break;
            }
        }
    }

    let _ = std::fs::remove_file(&path);
    Ok(())
}

fn plugdev_gid() -> Option<u32> {
    let content = std::fs::read_to_string("/etc/group").ok()?;
    for line in content.lines() {
        let mut parts = line.split(':');
        match parts.next() {
            Some("plugdev") => {}
            _ => continue,
        }
        let _ = parts.next(); // password
        if let Some(gid) = parts.next() {
            if let Ok(g) = gid.parse() {
                return Some(g);
            }
        }
    }
    None
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

        IpcCommand::SetRgbEffect {
            device_id,
            zone,
            effect,
            color,
        } => {
            let (vendor, name) = {
                let reg = state.registry.read().await;
                reg.device_vendor_name(&device_id)
            };
            let mut used_official = false;
            let mut official_err = None;
            match crate::drivers::set_effect_official(
                &vendor, &name, &zone, &effect, color.clone(),
            )
            .await
            {
                Ok(true) => used_official = true,
                Ok(false) => {}
                Err(e) => official_err = Some(e),
            }
            if used_official {
                IpcResponse::success(
                    id,
                    serde_json::json!({
                        "device_id": device_id,
                        "zone": zone,
                        "effect": effect,
                        "backend": "official",
                    }),
                )
            } else {
                let mut reg = state.registry.write().await;
                match reg.set_effect(&device_id, &zone, &effect, color) {
                    Ok(()) => IpcResponse::success(
                        id,
                        serde_json::json!({
                            "device_id": device_id,
                            "zone": zone,
                            "effect": effect,
                            "backend": "hid",
                            "official_error": official_err,
                        }),
                    ),
                    Err(e) => {
                        let msg = match official_err {
                            Some(oe) => format!("{e}; official: {oe}"),
                            None => e,
                        };
                        IpcResponse::failure(id, msg)
                    }
                }
            }
        }
        IpcCommand::SetColor {
            device_id,
            zone,
            color,
        } => {
            // Prefer official drivers (OpenRazer / OpenRGB)
            let (vendor, name) = {
                let reg = state.registry.read().await;
                reg.device_vendor_name(&device_id)
            };
            let mut used_official = false;
            let mut official_err = None;
            match crate::drivers::set_color_official(&vendor, &name, &zone, color.clone()).await {
                Ok(true) => used_official = true,
                Ok(false) => {}
                Err(e) => official_err = Some(e),
            }
            if used_official {
                IpcResponse::success(
                    id,
                    serde_json::json!({ "set": true, "backend": "official" }),
                )
            } else {
                let mut reg = state.registry.write().await;
                match reg.set_color(&device_id, &zone, color) {
                    Ok(()) => IpcResponse::success(
                        id,
                        serde_json::json!({
                            "set": true,
                            "backend": "hid",
                            "official_error": official_err,
                        }),
                    ),
                    Err(e) => {
                        let msg = match official_err {
                            Some(oe) => format!("{e}; official: {oe}"),
                            None => e,
                        };
                        IpcResponse::failure(id, msg)
                    }
                }
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


        IpcCommand::GetDeviceManager => {
            let dm = state.device_manager.read().await;
            let snap = crate::device_manager::snapshot(&dm).await;
            IpcResponse::success(id, snap)
        }

        IpcCommand::SetDeviceManager { state: new_state } => {
            let mut dm = state.device_manager.write().await;
            dm.set_state(new_state);
            match crate::device_manager::apply(&mut dm).await {
                Ok(v) => IpcResponse::success(id, v),
                Err(e) => IpcResponse::failure(id, e),
            }
        }

        IpcCommand::SetChannelDevicePolicy { policy } => {
            let mut dm = state.device_manager.write().await;
            dm.set_policy(policy);
            match crate::device_manager::apply(&mut dm).await {
                Ok(v) => IpcResponse::success(id, v),
                Err(e) => IpcResponse::failure(id, e),
            }
        }

        IpcCommand::LinkAllDevices { device_id, enabled } => {
            let mut dm = state.device_manager.write().await;
            dm.link_all(device_id, enabled);
            match crate::device_manager::apply(&mut dm).await {
                Ok(v) => IpcResponse::success(id, v),
                Err(e) => IpcResponse::failure(id, e),
            }
        }

        IpcCommand::ApplyDeviceManager => {
            let mut dm = state.device_manager.write().await;
            match crate::device_manager::apply(&mut dm).await {
                Ok(v) => IpcResponse::success(id, v),
                Err(e) => IpcResponse::failure(id, e),
            }
        }

        IpcCommand::ApplyAudioEffects { effects } => {
            match crate::dsp_pw::apply_effects(&effects).await {
                Ok(dsp) => IpcResponse::success(
                    id,
                    serde_json::json!({
                        "applied": dsp.applied,
                        "message": dsp.message,
                        "eq": dsp.eq,
                        "crossfeed": dsp.crossfeed,
                    }),
                ),
                Err(e) => IpcResponse::failure(id, e),
            }
        }

        IpcCommand::GetMicFx => {
            let state = crate::mic_fx::read_mic_snapshot().await;
            IpcResponse::success(id, serde_json::to_value(state).unwrap_or_default())
        }
        IpcCommand::SetMicFx { state } => {
            match crate::mic_fx::apply_mic_fx(&state).await {
                Ok(r) => IpcResponse::success(
                    id,
                    serde_json::json!({
                        "applied": r.applied,
                        "source": r.source,
                        "message": r.message,
                        "state": state,
                    }),
                ),
                Err(e) => IpcResponse::failure(id, e),
            }
        }

        IpcCommand::ImportJamesDspPreset {
            name,
            path,
            content,
            apply,
        } => {
            let preset = if let Some(p) = path {
                match crate::jamesdsp_import::import_file(std::path::Path::new(&p)) {
                    Ok(mut pr) => {
                        if let Some(n) = name {
                            pr.id = {
                                let s: String = n
                                    .chars()
                                    .map(|c| {
                                        if c.is_ascii_alphanumeric() {
                                            c.to_ascii_lowercase()
                                        } else {
                                            '-'
                                        }
                                    })
                                    .collect();
                                format!("jdsp-{}", s.trim_matches('-'))
                            };
                            pr.name = n;
                        }
                        pr
                    }
                    Err(e) => return IpcResponse::failure(id, e),
                }
            } else if let Some(c) = content {
                let n = name.unwrap_or_else(|| "JamesDSP Import".into());
                match crate::jamesdsp_import::import_string(&n, &c) {
                    Ok(pr) => pr,
                    Err(e) => return IpcResponse::failure(id, e),
                }
            } else {
                return IpcResponse::failure(id, "path or content required".to_string());
            };

            {
                let mut store = state.audio_presets.write().await;
                if let Err(e) = store.save(preset.clone()) {
                    return IpcResponse::failure(id, e);
                }
            }
            let mut applied = serde_json::Value::Null;
            if apply {
                match crate::audio_presets::apply_preset_runtime(&preset).await {
                    Ok(v) => applied = v,
                    Err(e) => {
                        return IpcResponse::success(
                            id,
                            serde_json::json!({
                                "preset": preset,
                                "saved": true,
                                "apply_error": e,
                            }),
                        );
                    }
                }
            }
            IpcResponse::success(
                id,
                serde_json::json!({
                    "preset": preset,
                    "saved": true,
                    "applied": applied,
                }),
            )
        }

        IpcCommand::ImportJamesDspDir { path, apply_first } => {
            match crate::jamesdsp_import::import_dir(std::path::Path::new(&path)) {
                Ok(list) => {
                    let mut ids = Vec::new();
                    {
                        let mut store = state.audio_presets.write().await;
                        for pr in &list {
                            if store.save(pr.clone()).is_ok() {
                                ids.push(pr.id.clone());
                            }
                        }
                    }
                    let mut applied = serde_json::Value::Null;
                    if apply_first {
                        if let Some(first) = list.first() {
                            if let Ok(v) =
                                crate::audio_presets::apply_preset_runtime(first).await
                            {
                                applied = v;
                            }
                        }
                    }
                    IpcResponse::success(
                        id,
                        serde_json::json!({
                            "imported": ids.len(),
                            "ids": ids,
                            "applied": applied,
                        }),
                    )
                }
                Err(e) => IpcResponse::failure(id, e),
            }
        }

        IpcCommand::GetMouseInfo { device_id } => {
            let (_, name) = {
                let reg = state.registry.read().await;
                reg.device_vendor_name(&device_id)
            };
            if !crate::drivers::ratbag::available().await {
                IpcResponse::failure(
                    id,
                    "ratbagctl not found — install piper/libratbag (ratbagd)".to_string(),
                )
            } else {
                match crate::drivers::ratbag::get_mouse_info(&name).await {
                    Ok(info) => IpcResponse::success(id, serde_json::to_value(info).unwrap_or_default()),
                    Err(e) => IpcResponse::failure(id, e),
                }
            }
        }

        IpcCommand::SetDpi {
            device_id,
            dpi,
            profile,
        } => {
            let (_, name) = {
                let reg = state.registry.read().await;
                reg.device_vendor_name(&device_id)
            };
            match crate::drivers::ratbag::set_dpi(&name, dpi, profile).await {
                Ok(msg) => IpcResponse::success(id, serde_json::json!({ "message": msg, "dpi": dpi })),
                Err(e) => IpcResponse::failure(id, e),
            }
        }

        IpcCommand::PrismSync {
            effect,
            zone,
            color,
        } => match crate::drivers::prism_sync(&effect, &zone, color).await {
            Ok(v) => IpcResponse::success(id, v),
            Err(e) => IpcResponse::failure(id, e),
        },

        IpcCommand::TakeScreenshot {
            mode,
            delay_secs,
            copy_clipboard,
            save_dir,
        } => {
            match crate::screenshot::capture(
                &mode,
                delay_secs,
                copy_clipboard,
                save_dir.as_deref(),
            )
            .await
            {
                Ok(r) => IpcResponse::success(
                    id,
                    serde_json::json!({
                        "path": r.path,
                        "backend": r.backend,
                        "message": r.message,
                    }),
                ),
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
