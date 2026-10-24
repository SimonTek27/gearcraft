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
