//! Blocking Unix-socket IPC client for GearCraft daemon.

use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::time::Duration;

use gearcraft_core::{
    default_socket_path, IpcCommand, IpcRequest, IpcResponse, RgbColor,
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
