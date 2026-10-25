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
// Audio presets
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EqBand {
    pub freq_hz: f32,
    pub gain_db: f32,
    pub q: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AudioEffects {
    #[serde(default)]
    pub eq_enabled: bool,
    #[serde(default)]
    pub eq_bands: Vec<EqBand>,
    #[serde(default)]
    pub bass_boost: bool,
    #[serde(default)]
    pub bass_boost_db: f32,
    #[serde(default)]
    pub compressor: bool,
    #[serde(default)]
    pub crossfeed: f32,
    #[serde(default)]
    pub limiter: bool,
    #[serde(default)]
    pub preamp_db: f32,
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
}

impl ChannelDevicePolicy {
    pub fn new(channel: SonarChannelId) -> Self {
        Self {
            channel,
            priority: Vec::new(),
            excluded: Vec::new(),
            active_device_id: None,
        }
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
