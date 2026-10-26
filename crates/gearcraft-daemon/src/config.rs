//! XDG config persistence for GearCraft daemon.

use std::fs;
use std::path::{Path, PathBuf};

use gearcraft_core::{AudioPreset, ChatMixState, DeviceManagerState, MacroDef, PresetRule};
use serde::{Deserialize, Serialize};
use tracing::warn;

pub fn config_dir() -> PathBuf {
    let dir = if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        PathBuf::from(xdg).join("gearcraft")
    } else if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join(".config").join("gearcraft")
    } else {
        PathBuf::from("/tmp/gearcraft-config")
    };
    if let Err(e) = fs::create_dir_all(&dir) {
        warn!("could not create config dir {}: {e}", dir.display());
    }
    dir
}

pub fn macros_path() -> PathBuf {
    config_dir().join("macros.json")
}

pub fn settings_path() -> PathBuf {
    config_dir().join("settings.json")
}

pub fn audio_presets_path() -> PathBuf {
    config_dir().join("audio_presets.json")
}

pub fn preset_rules_path() -> PathBuf {
    config_dir().join("preset_rules.json")
}

pub fn device_manager_path() -> PathBuf {
    config_dir().join("device_manager.json")
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Settings {
    #[serde(default)]
    pub chatmix: ChatMixState,
}

pub fn load_settings() -> Settings {
    let path = settings_path();
    match fs::read_to_string(&path) {
        Ok(data) => serde_json::from_str(&data).unwrap_or_default(),
        Err(_) => Settings::default(),
    }
}

#[allow(dead_code)]
pub fn save_settings(settings: &Settings) -> Result<(), String> {
    write_json(&settings_path(), settings)
}

pub fn load_macros() -> Option<Vec<MacroDef>> {
    let data = fs::read_to_string(macros_path()).ok()?;
    serde_json::from_str(&data).ok()
}

pub fn save_macros(macros: &[MacroDef]) -> Result<(), String> {
    write_json(&macros_path(), &macros)
}

pub fn load_audio_presets() -> Option<Vec<AudioPreset>> {
    let data = fs::read_to_string(audio_presets_path()).ok()?;
    serde_json::from_str(&data).ok()
}

pub fn save_audio_presets(presets: &[AudioPreset]) -> Result<(), String> {
    write_json(&audio_presets_path(), &presets)
}

pub fn load_preset_rules() -> Option<Vec<PresetRule>> {
    let data = fs::read_to_string(preset_rules_path()).ok()?;
    serde_json::from_str(&data).ok()
}

pub fn save_preset_rules(rules: &[PresetRule]) -> Result<(), String> {
    write_json(&preset_rules_path(), &rules)
}

pub fn load_device_manager() -> Option<DeviceManagerState> {
    let path = device_manager_path();
    let data = fs::read_to_string(&path).ok()?;
    match serde_json::from_str(&data) {
        Ok(v) => Some(v),
        Err(e) => {
            warn!("device_manager.json: {e}");
            None
        }
    }
}

pub fn save_device_manager(state: &DeviceManagerState) -> Result<(), String> {
    write_json(&device_manager_path(), state)
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let data = serde_json::to_string_pretty(value).map_err(|e| e.to_string())?;
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, data.as_bytes()).map_err(|e| e.to_string())?;
    fs::rename(&tmp, path).map_err(|e| e.to_string())?;
    Ok(())
}
