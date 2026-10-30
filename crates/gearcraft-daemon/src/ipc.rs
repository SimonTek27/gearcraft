//! Unix domain socket IPC server.
//!
//! Protocol: length-prefixed JSON (u32 LE length + UTF-8 JSON body).

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use gearcraft_core::{
    default_socket_path, ChatMixState, DeviceInfo, DeviceKind, IpcCommand, IpcRequest,
    IpcResponse, PeripheralAddon, RgbColor,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::RwLock;
use tracing::{error, info, warn};
use crate::addons;
use crate::device_manager;
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

    #[allow(dead_code)]
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

    /// Apply a named lighting effect through the raw-HID addon.
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

#[derive(Clone)]
pub struct SharedState {
    pub registry: Arc<RwLock<DeviceRegistry>>,
    pub macros: Arc<RwLock<MacroStore>>,
    pub chatmix: Arc<RwLock<ChatMixState>>,
    pub audio_presets: Arc<RwLock<crate::audio_presets::AudioPresetStore>>,
    pub device_manager: Arc<RwLock<crate::device_manager::DeviceManager>>,
}

pub async fn run_server(state: SharedState) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let sock = default_socket_path();
    if Path::new(&sock).exists() {
        let _ = std::fs::remove_file(&sock);
    }
    if let Some(parent) = Path::new(&sock).parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let listener = UnixListener::bind(&sock)?;
    // Make the socket reachable by members of the plugdev group (udev flow).
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&sock, std::fs::Permissions::from_mode(0o660));
        if let Some(gid) = plugdev_gid() {
            if let Some(p) = sock.to_str() {
                if let Ok(cpath) = std::ffi::CString::new(p) {
                    unsafe {
                        libc::chown(cpath.as_ptr(), u32::MAX, gid);
                    }
                }
            }
        }
    }
    info!("IPC listening on {}", sock.display());

    let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
    let mut sigint = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())?;

    loop {
        tokio::select! {
            res = listener.accept() => {
                match res {
                    Ok((stream, _)) => {
                        let st = state.clone();
                        tokio::spawn(async move {
                            if let Err(e) = handle_client(stream, st).await {
                                error!("client session error: {e}");
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

    let _ = std::fs::remove_file(&sock);
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
        if stream.read_exact(&mut len_buf).await.is_err() {
            break;
        }
        let len = u32::from_le_bytes(len_buf) as usize;
        if len == 0 || len > 4 * 1024 * 1024 {
            break;
        }
        let mut buf = vec![0u8; len];
        stream.read_exact(&mut buf).await?;
        let req: IpcRequest = match serde_json::from_slice(&buf) {
            Ok(r) => r,
            Err(e) => {
                warn!("bad request json: {e}");
                continue;
            }
        };
        let resp = dispatch(req, &state).await;
        let body = serde_json::to_vec(&resp)?;
        let lenb = (body.len() as u32).to_le_bytes();
        stream.write_all(&lenb).await?;
        stream.write_all(&body).await?;
        stream.flush().await?;
    }
    Ok(())
}

async fn dispatch(req: IpcRequest, state: &SharedState) -> IpcResponse {
    let id = req.id;
    match req.cmd {
        IpcCommand::GetDeviceManager => {
            let mgr = state.device_manager.read().await;
            IpcResponse::success(id, device_manager::snapshot(&mgr).await)
        }

        IpcCommand::SetDeviceManager { state: new_state } => {
            let mut mgr = state.device_manager.write().await;
            mgr.set_state(new_state);
            match device_manager::apply(&mut mgr).await {
                Ok(v) => IpcResponse::success(id, v),
                Err(e) => IpcResponse::failure(id, e),
            }
        }

        IpcCommand::SetChannelDevicePolicy { policy } => {
            let mut mgr = state.device_manager.write().await;
            mgr.set_policy(policy);
            let _ = mgr.save();
            match device_manager::apply(&mut mgr).await {
                Ok(v) => IpcResponse::success(id, v),
                Err(e) => IpcResponse::failure(id, e),
            }
        }

        IpcCommand::LinkAllDevices { device_id, enabled } => {
            let mut mgr = state.device_manager.write().await;
            mgr.link_all(device_id, enabled);
            let _ = mgr.save();
            match device_manager::apply(&mut mgr).await {
                Ok(v) => IpcResponse::success(id, v),
                Err(e) => IpcResponse::failure(id, e),
            }
        }

        IpcCommand::ApplyDeviceManager => {
            let mut mgr = state.device_manager.write().await;
            match device_manager::apply(&mut mgr).await {
                Ok(v) => IpcResponse::success(id, v),
                Err(e) => IpcResponse::failure(id, e),
            }
        }

        IpcCommand::Ping => IpcResponse::success(id, "pong"),

        IpcCommand::GetDevices => {
            let reg = state.registry.read().await;
            IpcResponse::success(id, reg.list())
        }

        IpcCommand::GetDeviceStatus { device_id } => {
            let reg = state.registry.read().await;
            match reg.status_json(&device_id) {
                Ok(v) => IpcResponse::success(id, v),
                Err(e) => IpcResponse::failure(id, e),
            }
        }

        IpcCommand::ApplyProfile { device_id, profile } => {
            let mut reg = state.registry.write().await;
            match reg.apply_profile(&device_id, profile) {
                Ok(()) => IpcResponse::success(id, serde_json::json!({ "ok": true })),
                Err(e) => IpcResponse::failure(id, e),
            }
        }

        IpcCommand::SetColor {
            device_id,
            zone,
            color,
        } => {
            // Official/system drivers first (OpenRazer → OpenRGB), raw HID as fallback.
            let (vendor, hint) = {
                let reg = state.registry.read().await;
                reg.get(&device_id)
                    .map(|d| (d.vendor.clone(), d.name.clone()))
                    .unwrap_or_else(|| (String::new(), device_id.clone()))
            };
            match crate::drivers::set_color_official(&vendor, &hint, &zone, color).await {
                Ok(true) => {
                    return IpcResponse::success(
                        id,
                        serde_json::json!({ "ok": true, "backend": "official" }),
                    )
                }
                Ok(false) => {}
                Err(e) => warn!("official backend failed ({e}) — falling back to HID"),
            }
            let mut reg = state.registry.write().await;
            match reg.set_color(&device_id, &zone, color) {
                Ok(()) => IpcResponse::success(
                    id,
                    serde_json::json!({ "ok": true, "backend": "hid" }),
                ),
                Err(e) => IpcResponse::failure(id, e),
            }
        }

        IpcCommand::ListAddons => IpcResponse::success(
            id,
            serde_json::json!(["steelseries", "razer"]),
        ),

        IpcCommand::ListMacros => {
            let macros = state.macros.read().await;
            IpcResponse::success(id, macros.list())
        }

        IpcCommand::UpsertMacro { macro_def } => {
            let mut macros = state.macros.write().await;
            macros.upsert(macro_def);
            IpcResponse::success(id, serde_json::json!({ "ok": true }))
        }

        IpcCommand::DeleteMacro { macro_id } => {
            let mut macros = state.macros.write().await;
            if macros.delete(&macro_id) {
                IpcResponse::success(id, serde_json::json!({ "deleted": true }))
            } else {
                IpcResponse::failure(id, format!("macro not found: {macro_id}"))
            }
        }

        IpcCommand::RunMacro { macro_id } => {
            let macros = state.macros.read().await;
            match macros.run(&macro_id) {
                Ok(_) => IpcResponse::success(id, serde_json::json!({ "ran": true })),
                Err(e) => IpcResponse::failure(id, e),
            }
        }

        IpcCommand::GetChatMix => {
            let cm = state.chatmix.read().await;
            IpcResponse::success(id, cm.clone())
        }

        IpcCommand::SetChatMix { balance } => {
            let mut cm = state.chatmix.write().await;
            cm.balance = balance.clamp(-100, 100);
            if cm.balance >= 0 {
                cm.game_volume = 100;
                cm.chat_volume = (100 - cm.balance) as u8;
            } else {
                cm.chat_volume = 100;
                cm.game_volume = (100 + cm.balance) as u8;
            }
            let g = cm.game_volume;
            let c = cm.chat_volume;
            let bal = cm.balance;
            drop(cm);
            match crate::chatmix_pw::apply_volumes(g, c).await {
                Ok(info) => IpcResponse::success(
                    id,
                    serde_json::json!({
                        "balance": bal,
                        "game_volume": g,
                        "chat_volume": c,
                        "backend": info.backend,
                        "message": info.message,
                    }),
                ),
                Err(e) => IpcResponse::failure(id, e),
            }
        }

        IpcCommand::SetupChatMix => match crate::chatmix_pw::setup_virtual_sinks().await {
            Ok(msg) => IpcResponse::success(id, serde_json::json!({ "message": msg })),
            Err(e) => IpcResponse::failure(id, e),
        },

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
                    if let Some(cm) = &p.chatmix {
                        let mut state_cm = state.chatmix.write().await;
                        *state_cm = cm.clone();
                    }
                    match crate::audio_presets::apply_preset_runtime(&p).await {
                        Ok(v) => IpcResponse::success(id, v),
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

        IpcCommand::SetEffect {
            device_id,
            zone,
            effect,
            color,
        } => {
            let (vendor, hint) = {
                let reg = state.registry.read().await;
                reg.get(&device_id)
                    .map(|d| (d.vendor.clone(), d.name.clone()))
                    .unwrap_or_else(|| (String::new(), device_id.clone()))
            };
            match crate::drivers::set_effect_official(&vendor, &hint, &zone, &effect, color).await
            {
                Ok(true) => IpcResponse::success(
                    id,
                    serde_json::json!({ "ok": true, "backend": "official" }),
                ),
                Ok(false) => {
                    let mut reg = state.registry.write().await;
                    match reg.set_effect(&device_id, &zone, &effect, color) {
                        Ok(()) => IpcResponse::success(
                            id,
                            serde_json::json!({ "ok": true, "backend": "hid" }),
                        ),
                        Err(e) => IpcResponse::failure(id, e),
                    }
                }
                Err(e) => IpcResponse::failure(id, e),
            }
        }

        IpcCommand::GetMouseInfo { device_id } => {
            let hint = product_hint(&state, &device_id).await;
            if !crate::drivers::ratbag::available().await {
                return IpcResponse::failure(
                    id,
                    "ratbagctl not available — install libratbag/piper and start ratbagd",
                );
            }
            match crate::drivers::ratbag::get_mouse_info(&hint).await {
                Ok(info) => IpcResponse::success(id, info),
                Err(e) => IpcResponse::failure(id, e),
            }
        }

        IpcCommand::SetDpi { device_id, dpi } => {
            let hint = product_hint(&state, &device_id).await;
            match crate::drivers::ratbag::set_dpi(&hint, dpi, 0).await {
                Ok(message) => {
                    IpcResponse::success(id, serde_json::json!({ "ok": true, "message": message }))
                }
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

        IpcCommand::GetMicFx => IpcResponse::success(id, crate::mic_fx::read_mic_snapshot().await),

        IpcCommand::SetMicFx { state: mic } => match crate::mic_fx::apply_mic_fx(&mic).await {
            Ok(r) => IpcResponse::success(
                id,
                serde_json::json!({
                    "applied": r.applied,
                    "source": r.source,
                    "message": r.message,
                }),
            ),
            Err(e) => IpcResponse::failure(id, e),
        },

        IpcCommand::ApplyAudioEffects { effects } => {
            match crate::dsp_pw::apply_effects(&effects).await {
                Ok(dsp) => IpcResponse::success(
                    id,
                    serde_json::json!({
                        "applied": dsp.applied,
                        "eq": dsp.eq,
                        "crossfeed": dsp.crossfeed,
                        "conf": dsp.conf_path,
                        "message": dsp.message,
                    }),
                ),
                Err(e) => IpcResponse::failure(id, e),
            }
        }

        IpcCommand::CaptureScreenshot {
            mode,
            delay_secs,
            clipboard,
        } => {
            match crate::screenshot::capture(&mode, delay_secs, clipboard, None).await {
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

        IpcCommand::ImportJamesDspPreset { path, apply } => {
            match crate::jamesdsp_import::import_file(Path::new(&path)) {
                Ok(preset) => {
                    let preset_id = preset.id.clone();
                    let mut store = state.audio_presets.write().await;
                    if let Err(e) = store.save(preset.clone()) {
                        return IpcResponse::failure(id, e);
                    }
                    drop(store);
                    if apply {
                        match crate::audio_presets::apply_preset_runtime(&preset).await {
                            Ok(v) => {
                                return IpcResponse::success(
                                    id,
                                    serde_json::json!({ "imported": preset_id, "applied": v }),
                                )
                            }
                            Err(e) => {
                                return IpcResponse::success(
                                    id,
                                    serde_json::json!({ "imported": preset_id, "apply_error": e }),
                                )
                            }
                        }
                    }
                    IpcResponse::success(id, serde_json::json!({ "imported": preset_id }))
                }
                Err(e) => IpcResponse::failure(id, e),
            }
        }

        IpcCommand::ImportJamesDspDir { path, apply_first } => {
            match crate::jamesdsp_import::import_dir(Path::new(&path)) {
                Ok(presets) => {
                    let mut first = None;
                    let mut saved = Vec::new();
                    let mut store = state.audio_presets.write().await;
                    for preset in presets {
                        let pid = preset.id.clone();
                        match store.save(preset.clone()) {
                            Ok(()) => {
                                if first.is_none() {
                                    first = Some(preset);
                                }
                                saved.push(pid);
                            }
                            Err(e) => warn!("import save failed: {e}"),
                        }
                    }
                    drop(store);
                    let mut applied = false;
                    if apply_first {
                        if let Some(preset) = first {
                            if crate::audio_presets::apply_preset_runtime(&preset).await.is_ok() {
                                applied = true;
                            }
                        }
                    }
                    IpcResponse::success(
                        id,
                        serde_json::json!({ "imported": saved, "applied": applied }),
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

/// Resolve the product hint used by official drivers (ratbag / OpenRazer / OpenRGB).
async fn product_hint(state: &SharedState, device_id: &str) -> String {
    let reg = state.registry.read().await;
    reg.get(device_id)
        .map(|d| d.name.clone())
        .unwrap_or_else(|| device_id.to_string())
}
