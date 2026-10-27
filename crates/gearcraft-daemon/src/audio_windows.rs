//! Windows audio backend placeholder (WASAPI / APO — not yet implemented).
//!
//! Linux uses PipeWire (`dsp_pw`, `chatmix_pw`, `app_routing`). On Windows those
//! modules should be `cfg(unix)` and this module documents the future API.

#![allow(dead_code)]

/// True when a real WASAPI path is available (currently always false).
pub fn wasapi_available() -> bool {
    false
}

pub fn setup_chatmix_stub() -> Result<String, String> {
    Err(
        "ChatMix/Sonar audio on Windows requires the WASAPI backend (not shipped yet). \
         Use the Linux build for full Sonar routing."
            .into(),
    )
}
