//! GearCraft core: shared types, addon trait and IPC protocol.
//!
//! The IPC layer uses JSON messages over a Unix domain socket.
//! Socket path: `$XDG_RUNTIME_DIR/gearcraft.sock` (fallback `/tmp/gearcraft.sock`).

use serde::{Deserialize, Serialize};
use std::fmt;

/// Vendor ID used by SteelSeries devices.
pub const VID_STEELSERIES: u16 = 0x1038;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DeviceKind {
    Mouse,
    Keyboard,
    Headset,
    Other,
}

impl fmt::Display for DeviceKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DeviceKind::Mouse => write!(f, "Mouse"),
            DeviceKind::Keyboard => write!(f, "Keyboard"),
            DeviceKind::Headset => write!(f, "Headset"),
            DeviceKind::Other => write!(f, "Other"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub id: String,
    pub name: String,
    pub vendor: String,
    pub kind: DeviceKind,
    pub vendor_id: u16,
    pub product_id: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RgbColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl RgbColor {
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    pub fn from_hex(hex: &str) -> Option<Self> {
        let hex = hex.trim_start_matches('#');
        if hex.len() != 6 {
            return None;
        }
        let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
        let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
        let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
        Some(Self { r, g, b })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DpiStage {
    pub stage: u8,
    pub dpi: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacroAction {
    pub key: String,
    pub down: bool,
    pub delay_ms: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacroDef {
    pub id: String,
    pub name: String,
    pub actions: Vec<MacroAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMixState {
    /// 0.0 = full game, 1.0 = full chat, 0.5 = balanced
    pub balance: f32,
    pub game_volume: f32,
    pub chat_volume: f32,
    pub enabled: bool,
}

impl Default for ChatMixState {
    fn default() -> Self {
        Self {
            balance: 0.5,
            game_volume: 1.0,
            chat_volume: 1.0,
            enabled: false,
        }
    }
}

/// Commands sent from UI → daemon over the Unix socket.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "cmd", content = "payload", rename_all = "snake_case")]
pub enum IpcCommand {
    ListDevices,
    GetDevice { id: String },
    SetColor { id: String, color: RgbColor },
    SetDpi { id: String, stages: Vec<DpiStage> },
    ListMacros,
    SaveMacro { macro_def: MacroDef },
    PlayMacro { id: String },
    DeleteMacro { id: String },
    GetChatMix,
    SetChatMix { state: ChatMixState },
    SetupChatMix,
    Ping,
}

/// Responses daemon → UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", content = "data", rename_all = "snake_case")]
pub enum IpcResponse {
    Ok,
    Devices(Vec<DeviceInfo>),
    Device(DeviceInfo),
    Macros(Vec<MacroDef>),
    ChatMix(ChatMixState),
    Error { message: String },
    Pong,
}

/// Trait that every brand/device family addon must implement.
pub trait PeripheralAddon: Send + Sync {
    fn name(&self) -> &str;
    fn vendor_id(&self) -> u16;
    fn supported_pids(&self) -> &[u16];
    fn probe(&self) -> Result<Vec<DeviceInfo>, Box<dyn std::error::Error + Send + Sync>>;
    fn set_color(
        &self,
        device: &DeviceInfo,
        color: &RgbColor,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    fn set_dpi(
        &self,
        device: &DeviceInfo,
        stages: &[DpiStage],
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
}
