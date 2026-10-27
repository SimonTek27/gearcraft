//! Windows audio backend placeholder (WASAPI / APO — not yet implemented).
#![allow(dead_code)]
pub fn wasapi_available() -> bool { false }
pub fn setup_chatmix_stub() -> Result<String, String> {
    Err("ChatMix/Sonar audio on Windows requires WASAPI backend (roadmap).".into())
}
