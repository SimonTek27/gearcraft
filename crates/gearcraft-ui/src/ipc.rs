//! Blocking Unix-socket IPC client for GearCraft daemon.

use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::time::Duration;

use gearcraft_core::{
    default_socket_path, AudioEffects, AudioPreset, MicFxState, ChannelDevicePolicy, IpcCommand, IpcRequest,
    IpcResponse, PresetRule, RgbColor,
};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug)]
pub enum IpcError {
    Connect(String),
    Io(String),
    Protocol(String),
    Remote(String),
}

impl std::fmt::Display for IpcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Connect(s) | Self::Io(s) | Self::Protocol(s) | Self::Remote(s) => {
                write!(f, "{s}")
            }
        }
    }
}

fn request(cmd: IpcCommand) -> Result<Value, IpcError> {
    let path = default_socket_path();
    let mut stream = UnixStream::connect(&path).map_err(|e| {
        IpcError::Connect(format!(
            "cannot connect to {} ({e}) — start gearcraft-daemon",
            path.display()
        ))
    })?;
    stream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .ok();
    stream
        .set_write_timeout(Some(Duration::from_secs(3)))
        .ok();

    let req = IpcRequest {
        id: Uuid::new_v4(),
        cmd,
    };
    let body = serde_json::to_vec(&req).map_err(|e| IpcError::Protocol(e.to_string()))?;
    let len = (body.len() as u32).to_le_bytes();
    stream
        .write_all(&len)
        .and_then(|_| stream.write_all(&body))
        .map_err(|e| IpcError::Io(e.to_string()))?;

    let mut len_buf = [0u8; 4];
    stream
        .read_exact(&mut len_buf)
        .map_err(|e| IpcError::Io(e.to_string()))?;
    let n = u32::from_le_bytes(len_buf) as usize;
    if n == 0 || n > 1_048_576 {
        return Err(IpcError::Protocol(format!("bad length {n}")));
    }
    let mut buf = vec![0u8; n];
    stream
        .read_exact(&mut buf)
        .map_err(|e| IpcError::Io(e.to_string()))?;

    let resp: IpcResponse =
        serde_json::from_slice(&buf).map_err(|e| IpcError::Protocol(e.to_string()))?;
    if !resp.ok {
        return Err(IpcError::Remote(
            resp.error.unwrap_or_else(|| "unknown error".into()),
        ));
    }
    Ok(resp.data.unwrap_or(Value::Null))
}

pub fn ping() -> Result<(), IpcError> {
    request(IpcCommand::Ping)?;
    Ok(())
}

pub fn get_devices() -> Result<Value, IpcError> {
    request(IpcCommand::GetDevices)
}

pub fn set_color(device_id: &str, zone: &str, r: u8, g: u8, b: u8) -> Result<(), IpcError> {
    request(IpcCommand::SetColor {
        device_id: device_id.to_string(),
        zone: zone.to_string(),
        color: RgbColor { r, g, b },
    })?;
    Ok(())
}

pub fn get_chatmix() -> Result<Value, IpcError> {
    request(IpcCommand::GetChatMix)
}

pub fn set_chatmix(balance: i8) -> Result<Value, IpcError> {
    request(IpcCommand::SetChatMix { balance })
}

pub fn setup_chatmix() -> Result<Value, IpcError> {
    request(IpcCommand::SetupChatMix)
}

pub fn list_audio_presets() -> Result<Value, IpcError> {
    request(IpcCommand::ListAudioPresets)
}

pub fn load_audio_preset(preset_id: &str) -> Result<Value, IpcError> {
    request(IpcCommand::LoadAudioPreset {
        preset_id: preset_id.to_string(),
    })
}

pub fn save_audio_preset(preset: AudioPreset) -> Result<Value, IpcError> {
    request(IpcCommand::SaveAudioPreset { preset })
}

pub fn delete_audio_preset(preset_id: &str) -> Result<Value, IpcError> {
    request(IpcCommand::DeleteAudioPreset {
        preset_id: preset_id.to_string(),
    })
}

pub fn list_preset_rules() -> Result<Value, IpcError> {
    request(IpcCommand::ListPresetRules)
}

pub fn set_preset_rule(rule: PresetRule) -> Result<Value, IpcError> {
    request(IpcCommand::SetPresetRule { rule })
}

pub fn delete_preset_rule(device_id: &str, route_id: &str) -> Result<Value, IpcError> {
    request(IpcCommand::DeletePresetRule {
        device_id: device_id.to_string(),
        route_id: route_id.to_string(),
    })
}


pub fn get_device_manager() -> Result<Value, IpcError> {
    request(IpcCommand::GetDeviceManager)
}

pub fn apply_device_manager() -> Result<Value, IpcError> {
    request(IpcCommand::ApplyDeviceManager)
}

pub fn link_all_devices(device_id: Option<String>, enabled: bool) -> Result<Value, IpcError> {
    request(IpcCommand::LinkAllDevices { device_id, enabled })
}

pub fn set_channel_device_policy(policy: ChannelDevicePolicy) -> Result<Value, IpcError> {
    request(IpcCommand::SetChannelDevicePolicy { policy })
}


pub fn apply_audio_effects(effects: AudioEffects) -> Result<Value, IpcError> {
    request(IpcCommand::ApplyAudioEffects { effects })
}


pub fn get_mic_fx() -> Result<Value, IpcError> {
    request(IpcCommand::GetMicFx)
}

pub fn set_mic_fx(state: MicFxState) -> Result<Value, IpcError> {
    request(IpcCommand::SetMicFx { state })
}


pub fn set_rgb_effect(
    device_id: &str,
    zone: &str,
    effect: &str,
    color: Option<RgbColor>,
) -> Result<(), IpcError> {
    request(IpcCommand::SetRgbEffect {
        device_id: device_id.to_string(),
        zone: zone.to_string(),
        effect: effect.to_string(),
        color,
    })?;
    Ok(())
}


pub fn take_screenshot(
    mode: &str,
    delay_secs: u8,
    copy_clipboard: bool,
) -> Result<Value, IpcError> {
    request(IpcCommand::TakeScreenshot {
        mode: mode.to_string(),
        delay_secs,
        copy_clipboard,
        save_dir: None,
    })
}


#[allow(dead_code)]
pub fn import_jamesdsp_path(path: &str, apply: bool) -> Result<Value, IpcError> {
    request(IpcCommand::ImportJamesDspPreset {
        name: None,
        path: Some(path.to_string()),
        content: None,
        apply,
    })
}

#[allow(dead_code)]
pub fn import_jamesdsp_dir(path: &str) -> Result<Value, IpcError> {
    request(IpcCommand::ImportJamesDspDir {
        path: path.to_string(),
        apply_first: false,
    })
}


pub fn get_mouse_info(device_id: &str) -> Result<Value, IpcError> {
    request(IpcCommand::GetMouseInfo {
        device_id: device_id.to_string(),
    })
}

pub fn set_dpi(device_id: &str, dpi: u32, profile: u32) -> Result<Value, IpcError> {
    request(IpcCommand::SetDpi {
        device_id: device_id.to_string(),
        dpi,
        profile,
    })
}

pub fn prism_sync(
    effect: &str,
    zone: &str,
    color: Option<RgbColor>,
) -> Result<Value, IpcError> {
    request(IpcCommand::PrismSync {
        effect: effect.to_string(),
        zone: zone.to_string(),
        color,
    })
}
