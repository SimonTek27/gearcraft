//! GearCraft UI – Tauri backend.
//!
//! Talks to the GearCraft daemon over the Unix socket defined in gearcraft-core.

use gearcraft_core::{
    default_socket_path, IpcCommand, IpcRequest, IpcResponse, MacroDef,
};
use serde_json::Value;
use tauri::Manager;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;
use uuid::Uuid;

async fn daemon_request(cmd: IpcCommand) -> Result<Value, String> {
    let path = default_socket_path();
    let mut stream = UnixStream::connect(&path)
        .await
        .map_err(|e| format!("cannot connect to daemon at {}: {e}", path.display()))?;

    let request = IpcRequest {
        id: Uuid::new_v4(),
        cmd,
    };
    let body = serde_json::to_vec(&request).map_err(|e| e.to_string())?;
    let len = (body.len() as u32).to_le_bytes();
    stream.write_all(&len).await.map_err(|e| e.to_string())?;
    stream.write_all(&body).await.map_err(|e| e.to_string())?;
    stream.flush().await.map_err(|e| e.to_string())?;

    let mut len_buf = [0u8; 4];
    stream
        .read_exact(&mut len_buf)
        .await
        .map_err(|e| e.to_string())?;
    let resp_len = u32::from_le_bytes(len_buf) as usize;
    if resp_len > 1_048_576 {
        return Err("response too large".into());
    }
    let mut resp_body = vec![0u8; resp_len];
    stream
        .read_exact(&mut resp_body)
        .await
        .map_err(|e| e.to_string())?;

    let response: IpcResponse =
        serde_json::from_slice(&resp_body).map_err(|e| e.to_string())?;

    if response.ok {
        Ok(response.data.unwrap_or(Value::Null))
    } else {
        Err(response.error.unwrap_or_else(|| "unknown error".into()))
    }
}

#[tauri::command]
async fn get_devices() -> Result<Value, String> {
    daemon_request(IpcCommand::GetDevices).await
}

#[tauri::command]
async fn get_device_status(device_id: String) -> Result<Value, String> {
    daemon_request(IpcCommand::GetDeviceStatus { device_id }).await
}

#[tauri::command]
async fn set_color(device_id: String, zone: String, r: u8, g: u8, b: u8) -> Result<Value, String> {
    daemon_request(IpcCommand::SetColor {
        device_id,
        zone,
        color: gearcraft_core::RgbColor { r, g, b },
    })
    .await
}

#[tauri::command]
async fn ping_daemon() -> Result<Value, String> {
    daemon_request(IpcCommand::Ping).await
}

#[tauri::command]
async fn list_macros() -> Result<Value, String> {
    daemon_request(IpcCommand::ListMacros).await
}

#[tauri::command]
async fn upsert_macro(macro_def: MacroDef) -> Result<Value, String> {
    daemon_request(IpcCommand::UpsertMacro { macro_def }).await
}

#[tauri::command]
async fn delete_macro(macro_id: String) -> Result<Value, String> {
    daemon_request(IpcCommand::DeleteMacro { macro_id }).await
}

#[tauri::command]
async fn run_macro(macro_id: String) -> Result<Value, String> {
    daemon_request(IpcCommand::RunMacro { macro_id }).await
}

#[tauri::command]
async fn get_chatmix() -> Result<Value, String> {
    daemon_request(IpcCommand::GetChatMix).await
}

#[tauri::command]
async fn set_chatmix(balance: i8) -> Result<Value, String> {
    daemon_request(IpcCommand::SetChatMix { balance }).await
}

#[tauri::command]
async fn setup_chatmix() -> Result<Value, String> {
    daemon_request(IpcCommand::SetupChatMix).await
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            get_devices,
            get_device_status,
            set_color,
            ping_daemon,
            list_macros,
            upsert_macro,
            delete_macro,
            run_macro,
            get_chatmix,
            set_chatmix,
            setup_chatmix,
        ])
        .setup(|app| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window;
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running GearCraft");
}
