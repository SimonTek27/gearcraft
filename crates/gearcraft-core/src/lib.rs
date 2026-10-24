//! GearCraft core: shared types, addon trait and IPC protocol.
//!
//! The IPC layer uses JSON messages over a Unix domain socket.
//! Socket path: `$XDG_RUNTIME_DIR/gearcraft.sock` (fallback `/tmp/gearcraft.sock`).

use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Device & status types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DeviceKind {
    Mouse,
    Keyboard,
    Headset,
    Mousepad,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    /// Stable unique id (usually vendor:product + serial or path)
    pub id: String,
    pub name: String,
    pub vendor: String,
    pub kind: DeviceKind,
    pub vendor_id: u16,
    pub product_id: u16,
    /// Path to hidraw device if available
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionStatus {
    Online,
    Offline,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceStatus {
    pub id: String,
    pub status: ConnectionStatus,
    /// Optional battery percentage 0-100
    pub battery: Option<u8>,
    /// Free-form status payload from the addon
    pub details: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RgbColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

// ---------------------------------------------------------------------------
// Addon trait
// ---------------------------------------------------------------------------

pub trait PeripheralAddon: Send + Sync {
    fn device_info(&self) -> DeviceInfo;
    fn initialize(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    fn apply_profile(&mut self, profile_data: serde_json::Value) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    fn get_status(&self) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>>;
    fn set_color(&mut self, zone: &str, color: RgbColor) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let _ = (zone, color);
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Macros & ChatMix
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacroAction {
    /// Key name or HID usage string, e.g. "KEY_F1", "ctrl+c"
    pub key: String,
    /// Delay after this action in ms
    #[serde(default)]
    pub delay_ms: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacroDef {
    pub id: String,
    pub name: String,
    /// Device this macro is bound to (optional = global)
    #[serde(default)]
    pub device_id: Option<String>,
    /// Trigger button / key label
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
    /// -100 = full chat, 0 = balanced, +100 = full game
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
// Audio presets (JDSP4Linux-style — GearCraft aims to replace JamesDSP)
// ---------------------------------------------------------------------------

/// One graphic-EQ band (Hz + gain dB).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EqBand {
    pub freq_hz: f32,
    /// Gain in dB, typically −12…+12
    pub gain_db: f32,
}

fn default_eq_bands() -> Vec<EqBand> {
    [31.5, 63.0, 125.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0, 16000.0]
        .into_iter()
        .map(|freq_hz| EqBand {
            freq_hz,
            gain_db: 0.0,
        })
        .collect()
}

/// Effect toggles / parameters stored with a preset.
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
    pub limiter: bool,
    #[serde(default = "default_limiter_db")]
    pub limiter_db: f32,
    #[serde(default)]
    pub bypass: bool,
}

fn default_limiter_db() -> f32 {
    -0.1
}

impl Default for AudioEffects {
    fn default() -> Self {
        Self {
            eq_enabled: false,
            eq_bands: default_eq_bands(),
            bass_boost: false,
            bass_boost_db: 0.0,
            compressor: false,
            reverb: false,
            crossfeed: false,
            crossfeed_strength: 0,
            wideness: false,
            wideness_pct: 0,
            limiter: true,
            limiter_db: -0.1,
            bypass: false,
        }
    }
}

/// Named audio preset (EQ + effects + optional ChatMix snapshot).
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

/// Auto-apply a preset when a given output / headset is active.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresetRule {
    pub device_id: String,
    #[serde(default)]
    pub device_name: String,
    #[serde(default)]
    pub route_id: String,
    pub preset_id: String,
}

// ---------------------------------------------------------------------------
// IPC Protocol
// ---------------------------------------------------------------------------

/// Default Unix socket path.
pub fn default_socket_path() -> std::path::PathBuf {
    if let Ok(runtime) = std::env::var("XDG_RUNTIME_DIR") {
        std::path::PathBuf::from(runtime).join("gearcraft.sock")
    } else {
        std::path::PathBuf::from("/tmp/gearcraft.sock")
    }
}

/// Client → Daemon request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcRequest {
    /// Correlation id (UUID)
    pub id: Uuid,
    pub cmd: IpcCommand,
}

/// Daemon → Client response
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

/// All supported IPC commands.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "name", content = "params", rename_all = "snake_case")]
pub enum IpcCommand {
    /// List all known / connected devices
    GetDevices,

    /// Get detailed status for one device
    GetDeviceStatus { device_id: String },

    /// Apply a full profile (JSON blob defined by the addon)
    ApplyProfile {
        device_id: String,
        profile: serde_json::Value,
    },

    /// Quick RGB colour change for a zone
    SetColor {
        device_id: String,
        zone: String,
        color: RgbColor,
    },

    /// List loaded addons
    ListAddons,

    /// List saved macros
    ListMacros,

    /// Create or update a macro
    UpsertMacro { macro_def: MacroDef },

    /// Delete a macro by id
    DeleteMacro { macro_id: String },

    /// Fire a macro once (test / runtime)
    RunMacro { macro_id: String },

    /// Get software ChatMix state
    GetChatMix,

    /// Set software ChatMix balance (-100..=100)
    SetChatMix { balance: i8 },

    /// Create PipeWire/Pulse null sinks for Game/Chat
    SetupChatMix,

    /// List saved audio presets (JDSP-style)
    ListAudioPresets,

    /// Load audio preset by id and apply (ChatMix + WirePlumber DSP)
    LoadAudioPreset { preset_id: String },

    /// Save / upsert audio preset
    SaveAudioPreset { preset: AudioPreset },

    /// Delete audio preset by id
    DeleteAudioPreset { preset_id: String },

    /// List device→preset auto-apply rules
    ListPresetRules,

    /// Add or update a preset rule
    SetPresetRule { rule: PresetRule },

    /// Delete preset rule for a device (+ optional route)
    DeletePresetRule {
        device_id: String,
        #[serde(default)]
        route_id: String,
    },

    /// Ping / health check
    Ping,

    /// Graceful shutdown of the daemon (admin only, future)
    Shutdown,
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("device not found: {0}")]
    DeviceNotFound(String),
    #[error("addon error: {0}")]
    Addon(String),
    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}
