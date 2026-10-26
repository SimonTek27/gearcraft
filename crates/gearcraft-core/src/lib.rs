//! GearCraft core: shared types, addon trait and IPC protocol.
//!
//! The IPC layer uses JSON messages over a Unix domain socket.
//! Socket path: `$XDG_RUNTIME_DIR/gearcraft.sock` (fallback `/run/gearcraft/gearcraft.sock`,
//! overridable via `GEARCRAFT_SOCKET`).

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
    /// RGB effect name: static | rainbow | breathe | spectrum | wave | reactive
    fn set_effect(
        &mut self,
        zone: &str,
        effect: &str,
        color: Option<RgbColor>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if effect == "static" {
            if let Some(c) = color {
                return self.set_color(zone, c);
            }
        }
        let _ = (zone, effect, color);
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

/// One graphic-EQ band (Hz + gain dB). JamesDSP uses FIR/IIR multimodal EQ;
/// we persist a portable band list that can drive PipeWire / EasyEffects later.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EqBand {
    pub freq_hz: f32,
    /// Gain in dB, typically −12…+12
    pub gain_db: f32,
}

fn default_eq_bands() -> Vec<EqBand> {
    // 10-band ISO-ish centres
    [31.5, 63.0, 125.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0, 16000.0]
        .into_iter()
        .map(|freq_hz| EqBand {
            freq_hz,
            gain_db: 0.0,
        })
        .collect()
}

/// Effect toggles / parameters stored with a preset (DSP engine TBD).
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
    /// 0…100 compressor amount (ratio / threshold map)
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
    /// 0 = off … 100 = strong (BS2B-style)
    #[serde(default)]
    pub crossfeed_strength: u8,
    #[serde(default)]
    pub wideness: bool,
    #[serde(default)]
    pub wideness_pct: u8,
    /// Sonar spatial: 0 = performance (FPS), 100 = immersion (single-player)
    #[serde(default)]
    pub spatial_enabled: bool,
    #[serde(default)]
    pub spatial_immersion: u8,
    /// Virtual speaker distance 0 (near) … 100 (far)
    #[serde(default)]
    pub spatial_distance: u8,
    /// Pre-DSP volume boost in dB (Sonar "volume boost")
    #[serde(default)]
    pub volume_boost_db: f32,
    /// Gentle auto-gain style compression (approx)
    #[serde(default)]
    pub smart_volume: bool,
    #[serde(default)]
    pub limiter: bool,
    #[serde(default = "default_limiter_db")]
    pub limiter_db: f32,
    #[serde(default)]
    pub bypass: bool,
    /// Convolution reverb / headphone correction (IR file path)
    #[serde(default)]
    pub convolver_enable: bool,
    #[serde(default)]
    pub convolver_file: String,
    #[serde(default)]
    pub convolver_mix: u8,
}

fn default_limiter_db() -> f32 {
    -0.1
}
fn default_lowpass_hz() -> f32 {
    12000.0
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

/// Microphone processing (Sonar Mic tab — simplified).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MicFxState {
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Input gain 0…150 (%)
    #[serde(default = "default_mic_gain")]
    pub gain_pct: u8,
    /// High-pass / rumble filter (~80–120 Hz)
    #[serde(default)]
    pub highpass: bool,
    #[serde(default = "default_highpass_hz")]
    pub highpass_hz: f32,
    /// Soft noise gate (threshold approx)
    #[serde(default)]
    pub noise_gate: bool,
    /// 0…100 — higher = more aggressive gate
    #[serde(default = "default_gate")]
    pub gate_strength: u8,
    /// Low-pass (hiss / air cut)
    #[serde(default)]
    pub lowpass: bool,
    #[serde(default = "default_mic_lowpass_hz")]
    pub lowpass_hz: f32,
    /// Real dynamics compressor
    #[serde(default)]
    pub compressor: bool,
    #[serde(default = "default_gate")]
    pub compressor_strength: u8,
    /// RNNoise-style noise suppression (PipeWire/LADSPA if available)
    #[serde(default)]
    pub rnnoise: bool,
    #[serde(default)]
    pub muted: bool,
}

fn default_mic_lowpass_hz() -> f32 {
    10000.0
}

fn default_mic_gain() -> u8 {
    100
}
fn default_highpass_hz() -> f32 {
    100.0
}
fn default_gate() -> u8 {
    40
}

impl Default for MicFxState {
    fn default() -> Self {
        Self {
            enabled: true,
            gain_pct: 100,
            highpass: true,
            highpass_hz: 100.0,
            noise_gate: false,
            gate_strength: 40,
            lowpass: false,
            lowpass_hz: 10000.0,
            compressor: false,
            compressor_strength: 40,
            rnnoise: false,
            muted: false,
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
    /// Optional ChatMix snapshot applied on load
    #[serde(default)]
    pub chatmix: Option<ChatMixState>,
    /// Free-form notes
    #[serde(default)]
    pub notes: String,
}

/// Auto-apply a preset when a given output / headset is active (JDSP rules).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresetRule {
    /// Device id (GearCraft device id or PipeWire sink name)
    pub device_id: String,
    #[serde(default)]
    pub device_name: String,
    /// Optional route (e.g. headphone vs speaker on same card)
    #[serde(default)]
    pub route_id: String,
    pub preset_id: String,
}

// ---------------------------------------------------------------------------
// IPC Protocol
// ---------------------------------------------------------------------------

/// Default Unix socket path.
pub fn default_socket_path() -> std::path::PathBuf {
    if let Ok(p) = std::env::var("GEARCRAFT_SOCKET") {
        return std::path::PathBuf::from(p);
    }
    if let Ok(runtime) = std::env::var("XDG_RUNTIME_DIR") {
        std::path::PathBuf::from(runtime).join("gearcraft.sock")
    } else if std::path::Path::new("/run/gearcraft").exists() {
        // System service: shared location (created by systemd RuntimeDirectory
        // or by the daemon when running as root).
        std::path::PathBuf::from("/run/gearcraft/gearcraft.sock")
    } else {
        // Dev / non-root fallback.
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

// ---------------------------------------------------------------------------
// Sonar Device Manager (physical output per channel + priority/backup)
// ---------------------------------------------------------------------------

/// Logical Sonar mixer channel that can target a physical sink.
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

/// One physical (or PipeWire) sink entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioOutputDevice {
    /// Stable id: pactl name or wpctl id string
    pub id: String,
    pub name: String,
    pub description: String,
    /// True for GearCraft virtual sinks (usually not selected as physical targets)
    pub is_virtual: bool,
    pub available: bool,
}

/// Per-channel device policy (priority list + exclusions).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelDevicePolicy {
    pub channel: SonarChannelId,
    /// Ordered preferred physical sink ids (first available wins)
    #[serde(default)]
    pub priority: Vec<String>,
    /// Sink ids never used for this channel
    #[serde(default)]
    pub excluded: Vec<String>,
    /// Currently applied physical sink id (runtime)
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
    /// Same physical device forced on all output channels
    #[serde(default)]
    pub link_all: bool,
    #[serde(default)]
    pub backup_mode: BackupMode,
    /// On daemon start, if primary missing, pick next priority
    #[serde(default = "default_true_dm")]
    pub auto_switch_on_startup: bool,
    pub channels: Vec<ChannelDevicePolicy>,
}

fn default_true_dm() -> bool {
    true
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
    /// Prism: set lighting effect (static/rainbow/breathe/…)
    SetRgbEffect {
        device_id: String,
        zone: String,
        effect: String,
        color: Option<RgbColor>,
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

    /// Load audio preset by id and apply (ChatMix + effects state)
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

    /// List physical + virtual audio outputs and Device Manager state
    GetDeviceManager,

    /// Replace full Device Manager state and re-apply routing
    SetDeviceManager { state: DeviceManagerState },

    /// Set priority/exclusion for one channel
    SetChannelDevicePolicy { policy: ChannelDevicePolicy },

    /// Link the same primary device to all channels (or unlink)
    LinkAllDevices { device_id: Option<String>, enabled: bool },

    /// Force re-apply loopbacks / failover
    ApplyDeviceManager,
    /// Live-apply DSP effects (EQ / spatial / boost) without saving a preset
    ApplyAudioEffects { effects: AudioEffects },
    GetMicFx,
    SetMicFx { state: MicFxState },
    /// Import a JamesDSP .conf preset (path on daemon host, or inline content)
    ImportJamesDspPreset {
        /// Display name (optional; defaults to file stem)
        #[serde(default)]
        name: Option<String>,
        /// Absolute path to .conf on the machine running the daemon
        #[serde(default)]
        path: Option<String>,
        /// Inline conf body (alternative to path)
        #[serde(default)]
        content: Option<String>,
        /// Also apply DSP immediately after import
        #[serde(default)]
        apply: bool,
    },
    /// Import all .conf from a directory (e.g. ~/.config/jamesdsp/presets)
    ImportJamesDspDir {
        path: String,
        #[serde(default)]
        apply_first: bool,
    },
    /// Mouse DPI via libratbag/ratbagctl (Piper stack)
    GetMouseInfo { device_id: String },
    SetDpi {
        device_id: String,
        dpi: u32,
        #[serde(default)]
        profile: u32,
    },
    /// Apply same RGB to all known devices (PrismSync-lite)
    PrismSync {
        effect: String,
        zone: String,
        color: Option<RgbColor>,
    },
    /// Screen capture (Spectacle-style)
    TakeScreenshot {
        /// full | window | region
        mode: String,
        /// delay seconds before capture
        #[serde(default)]
        delay_secs: u8,
        /// copy image to clipboard after capture
        #[serde(default = "default_true")]
        copy_clipboard: bool,
        /// optional save directory (default XDG pictures/gearcraft)
        #[serde(default)]
        save_dir: Option<String>,
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
