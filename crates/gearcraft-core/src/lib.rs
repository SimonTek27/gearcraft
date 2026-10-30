//! GearCraft core: shared types, addon trait and IPC protocol.
//!
//! The IPC layer uses JSON messages over a Unix domain socket.
//! Socket path: `$XDG_RUNTIME_DIR/gearcraft.sock` (fallback `/tmp/gearcraft.sock`).

use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Devices
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DeviceKind {
    Mouse,
    Keyboard,
    Headset,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub id: String,
    pub name: String,
    pub vendor: String,
    pub kind: DeviceKind,
    pub vendor_id: u16,
    pub product_id: u16,
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionStatus {
    Online,
    Offline,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceStatus {
    pub device_id: String,
    pub connection: ConnectionStatus,
    pub battery_percent: Option<u8>,
    pub details: serde_json::Value,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct RgbColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

pub trait PeripheralAddon: Send + Sync {
    fn device_info(&self) -> DeviceInfo;
    fn initialize(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    fn apply_profile(
        &mut self,
        profile_data: serde_json::Value,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    fn get_status(&self) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>>;
    fn set_color(
        &mut self,
        zone: &str,
        color: RgbColor,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let _ = (zone, color);
        Ok(())
    }

    /// Apply a named lighting effect (static/rainbow/breathe/…). Default: no-op.
    fn set_effect(
        &mut self,
        _zone: &str,
        _effect: &str,
        _color: Option<RgbColor>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Macros & ChatMix
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacroAction {
    pub key: String,
    #[serde(default)]
    pub delay_ms: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacroDef {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub device_id: Option<String>,
    pub trigger: String,
    pub actions: Vec<MacroAction>,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMixState {
    pub balance: i8,
    pub game_volume: u8,
    pub chat_volume: u8,
}

impl Default for ChatMixState {
    fn default() -> Self {
        Self {
            balance: 0,
            game_volume: 100,
            chat_volume: 100,
        }
    }
}

// ---------------------------------------------------------------------------
// Microphone FX
// ---------------------------------------------------------------------------

/// Microphone chain state (gain / mute / high-pass / gate …).
/// Applied by the daemon through `pactl` (and the WirePlumber mic filter chain).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MicFxState {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub muted: bool,
    #[serde(default = "default_mic_gain")]
    pub gain_pct: u8,
    #[serde(default = "default_true")]
    pub highpass: bool,
    #[serde(default = "default_highpass_hz")]
    pub highpass_hz: f32,
    #[serde(default)]
    pub noise_gate: bool,
    #[serde(default = "default_gate_strength")]
    pub gate_strength: u8,
    #[serde(default)]
    pub lowpass: bool,
    #[serde(default)]
    pub compressor: bool,
    #[serde(default)]
    pub rnnoise: bool,
}

impl Default for MicFxState {
    fn default() -> Self {
        Self {
            enabled: true,
            muted: false,
            gain_pct: default_mic_gain(),
            highpass: true,
            highpass_hz: default_highpass_hz(),
            noise_gate: false,
            gate_strength: default_gate_strength(),
            lowpass: false,
            compressor: false,
            rnnoise: false,
        }
    }
}

fn default_mic_gain() -> u8 {
    100
}

fn default_highpass_hz() -> f32 {
    80.0
}

fn default_gate_strength() -> u8 {
    40
}

// ---------------------------------------------------------------------------
// Audio presets
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EqBand {
    pub freq_hz: f32,
    pub gain_db: f32,
}

fn default_eq_bands() -> Vec<EqBand> {
    [31.5, 63.0, 125.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0, 16000.0]
        .into_iter()
        .map(|freq_hz| EqBand { freq_hz, gain_db: 0.0 })
        .collect()
}

fn default_lowpass_hz() -> f32 {
    12000.0
}

fn default_limiter_db() -> f32 {
    -0.1
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioEffects {
    #[serde(default)]
    pub eq_enabled: bool,
    #[serde(default = "default_eq_bands")]
    pub eq_bands: Vec<EqBand>,
    #[serde(default)]
    pub bass_boost: bool,
    #[serde(default)]
    pub bass_boost_db: f32,
    #[serde(default)]
    pub compressor: bool,
    #[serde(default)]
    pub compressor_strength: u8,
    #[serde(default)]
    pub lowpass: bool,
    #[serde(default = "default_lowpass_hz")]
    pub lowpass_hz: f32,
    #[serde(default)]
    pub reverb: bool,
    #[serde(default)]
    pub crossfeed: bool,
    #[serde(default)]
    pub crossfeed_strength: u8,
    #[serde(default)]
    pub wideness: bool,
    #[serde(default)]
    pub wideness_pct: u8,
    #[serde(default)]
    pub spatial_enabled: bool,
    #[serde(default)]
    pub spatial_immersion: u8,
    #[serde(default)]
    pub spatial_distance: u8,
    #[serde(default)]
    pub volume_boost_db: f32,
    #[serde(default)]
    pub smart_volume: bool,
    #[serde(default)]
    pub limiter: bool,
    #[serde(default = "default_limiter_db")]
    pub limiter_db: f32,
    #[serde(default)]
    pub bypass: bool,
    #[serde(default)]
    pub convolver_enable: bool,
    #[serde(default)]
    pub convolver_file: String,
    #[serde(default)]
    pub convolver_mix: u8,
}

impl Default for AudioEffects {
    fn default() -> Self {
        Self {
            eq_enabled: false,
            eq_bands: default_eq_bands(),
            bass_boost: false,
            bass_boost_db: 0.0,
            compressor: false,
            compressor_strength: 40,
            lowpass: false,
            lowpass_hz: 12000.0,
            reverb: false,
            crossfeed: false,
            crossfeed_strength: 0,
            wideness: false,
            wideness_pct: 0,
            spatial_enabled: false,
            spatial_immersion: 40,
            spatial_distance: 40,
            volume_boost_db: 0.0,
            smart_volume: false,
            limiter: true,
            limiter_db: -0.1,
            bypass: false,
            convolver_enable: false,
            convolver_file: String::new(),
            convolver_mix: 100,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioPreset {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub effects: AudioEffects,
    #[serde(default)]
    pub chatmix: Option<ChatMixState>,
    #[serde(default)]
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresetRule {
    pub device_id: String,
    #[serde(default)]
    pub route_id: String,
    pub preset_id: String,
}

// ---------------------------------------------------------------------------
// Sonar Device Manager
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SonarChannelId {
    Game,
    Chat,
    Media,
    Aux,
}

impl SonarChannelId {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Game => "game",
            Self::Chat => "chat",
            Self::Media => "media",
            Self::Aux => "aux",
        }
    }

    pub fn all() -> [Self; 4] {
        [Self::Game, Self::Chat, Self::Media, Self::Aux]
    }

    pub fn virtual_sink(self) -> &'static str {
        match self {
            Self::Game => "gearcraft_game",
            Self::Chat => "gearcraft_chat",
            Self::Media => "gearcraft_media",
            Self::Aux => "gearcraft_aux",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioOutputDevice {
    pub id: String,
    pub name: String,
    pub description: String,
    pub is_virtual: bool,
    pub available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelDevicePolicy {
    pub channel: SonarChannelId,
    #[serde(default)]
    pub priority: Vec<String>,
    #[serde(default)]
    pub excluded: Vec<String>,
    #[serde(default)]
    pub active_device_id: Option<String>,
    #[serde(default)]
    pub primary_device_id: Option<String>,
    #[serde(default)]
    pub backup_device_id: Option<String>,
    #[serde(default)]
    pub hotkey_volume_up: String,
    #[serde(default)]
    pub hotkey_volume_down: String,
    #[serde(default)]
    pub hotkey_mute: String,
}

impl ChannelDevicePolicy {
    pub fn new(channel: SonarChannelId) -> Self {
        Self {
            channel,
            priority: Vec::new(),
            excluded: Vec::new(),
            active_device_id: None,
            primary_device_id: None,
            backup_device_id: None,
            hotkey_volume_up: String::new(),
            hotkey_volume_down: String::new(),
            hotkey_mute: String::new(),
        }
    }

    pub fn sync_priority_from_primary_backup(&mut self) {
        let mut p = Vec::new();
        if let Some(ref id) = self.primary_device_id {
            if !id.is_empty() {
                p.push(id.clone());
            }
        }
        if let Some(ref id) = self.backup_device_id {
            if !id.is_empty() && !p.iter().any(|x| x == id) {
                p.push(id.clone());
            }
        }
        for id in &self.priority {
            if !p.iter().any(|x| x == id) {
                p.push(id.clone());
            }
        }
        self.priority = p;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackupMode {
    Manual,
    Automatic,
}

impl Default for BackupMode {
    fn default() -> Self {
        Self::Automatic
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceManagerState {
    #[serde(default)]
    pub link_all: bool,
    #[serde(default)]
    pub backup_mode: BackupMode,
    #[serde(default = "default_true")]
    pub auto_switch_on_startup: bool,
    pub channels: Vec<ChannelDevicePolicy>,
}

impl Default for DeviceManagerState {
    fn default() -> Self {
        Self {
            link_all: true,
            backup_mode: BackupMode::Automatic,
            auto_switch_on_startup: true,
            channels: SonarChannelId::all()
                .into_iter()
                .map(ChannelDevicePolicy::new)
                .collect(),
        }
    }
}

// ---------------------------------------------------------------------------
// IPC
// ---------------------------------------------------------------------------

pub fn default_socket_path() -> std::path::PathBuf {
    if let Ok(runtime) = std::env::var("XDG_RUNTIME_DIR") {
        std::path::PathBuf::from(runtime).join("gearcraft.sock")
    } else {
        std::path::PathBuf::from("/tmp/gearcraft.sock")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcRequest {
    pub id: Uuid,
    pub cmd: IpcCommand,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcResponse {
    pub id: Uuid,
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl IpcResponse {
    pub fn success(id: Uuid, data: impl Serialize) -> Self {
        Self {
            id,
            ok: true,
            data: Some(serde_json::to_value(data).unwrap_or(serde_json::Value::Null)),
            error: None,
        }
    }

    pub fn failure(id: Uuid, error: impl Into<String>) -> Self {
        Self {
            id,
            ok: false,
            data: None,
            error: Some(error.into()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum IpcCommand {
    GetDevices,
    GetDeviceStatus { device_id: String },
    ApplyProfile {
        device_id: String,
        profile: serde_json::Value,
    },
    SetColor {
        device_id: String,
        zone: String,
        color: RgbColor,
    },
    ListAddons,
    ListMacros,
    UpsertMacro { macro_def: MacroDef },
    DeleteMacro { macro_id: String },
    RunMacro { macro_id: String },
    GetChatMix,
    SetChatMix { balance: i8 },
    SetupChatMix,
    ListAudioPresets,
    LoadAudioPreset { preset_id: String },
    SaveAudioPreset { preset: AudioPreset },
    DeleteAudioPreset { preset_id: String },
    ListPresetRules,
    SetPresetRule { rule: PresetRule },
    DeletePresetRule {
        device_id: String,
        #[serde(default)]
        route_id: String,
    },
    GetMouseInfo { device_id: String },
    SetDpi { device_id: String, dpi: u32 },
    PrismSync {
        effect: String,
        zone: String,
        #[serde(default)]
        color: Option<RgbColor>,
    },
    SetEffect {
        device_id: String,
        zone: String,
        effect: String,
        #[serde(default)]
        color: Option<RgbColor>,
    },
    GetMicFx,
    SetMicFx { state: MicFxState },
    ApplyAudioEffects { effects: AudioEffects },
    CaptureScreenshot {
        mode: String,
        #[serde(default)]
        delay_secs: u8,
        #[serde(default)]
        clipboard: bool,
    },
    ImportJamesDspPreset {
        path: String,
        #[serde(default)]
        apply: bool,
    },
    ImportJamesDspDir {
        path: String,
        #[serde(default)]
        apply_first: bool,
    },
    GetDeviceManager,
    SetDeviceManager { state: DeviceManagerState },
    SetChannelDevicePolicy { policy: ChannelDevicePolicy },
    LinkAllDevices {
        device_id: Option<String>,
        enabled: bool,
    },
    ApplyDeviceManager,
    Ping,
    Shutdown,
}

#[derive(Debug, Error)]
pub enum GearCraftError {
    #[error("{0}")]
    Message(String),
}
