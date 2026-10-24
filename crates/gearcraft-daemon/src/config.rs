//! XDG config persistence for GearCraft daemon.
//!
//! Default directory: `$XDG_CONFIG_HOME/gearcraft` or `~/.config/gearcraft`
//!
//! | File | Content |
//! |------|---------|
//! | `macros.json` | Array of [`MacroDef`] |
//! | `settings.json` | ChatMix balance and future preferences |
//! | `audio_presets.json` | Named EQ/effects presets (JDSP-style) |
//! | `preset_rules.json` | Device → preset auto-apply rules |

use std::fs;
use std::path::{Path, PathBuf};

use gearcraft_core::{AudioPreset, ChatMixState, MacroDef, PresetRule};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

/// Resolve the GearCraft config directory (creates it if missing).
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

// ---------------------------------------------------------------------------
// Settings (ChatMix + future prefs)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default)]
    pub chatmix: ChatMixState,
    /// Schema version for future migrations
    #[serde(default = "default_version")]
    pub version: u32,
}

fn default_version() -> u32 {
    1
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            chatmix: ChatMixState::default(),
            version: 1,
        }
    }
}

pub fn load_settings() -> Settings {
    let path = settings_path();
    match read_json::<Settings>(&path) {
        Ok(s) => {
            info!("loaded settings from {}", path.display());
            s
        }
        Err(e) => {
            info!("no settings yet ({e}); using defaults");
            Settings::default()
        }
    }
}

pub fn save_settings(settings: &Settings) -> Result<(), String> {
    write_json(&settings_path(), settings)
}

pub fn save_chatmix(cm: &ChatMixState) -> Result<(), String> {
    let mut s = load_settings();
    s.chatmix = cm.clone();
    save_settings(&s)
}

// ---------------------------------------------------------------------------
// Macros
// ---------------------------------------------------------------------------

pub fn load_macros() -> Option<Vec<MacroDef>> {
    let path = macros_path();
    match read_json::<Vec<MacroDef>>(&path) {
        Ok(list) => {
            info!("loaded {} macro(s) from {}", list.len(), path.display());
            Some(list)
        }
        Err(e) => {
            info!("no macros file yet ({e})");
            None
        }
    }
}

pub fn save_macros(macros: &[MacroDef]) -> Result<(), String> {
    write_json(&macros_path(), &macros)
}

// ---------------------------------------------------------------------------
// Audio presets & rules
// ---------------------------------------------------------------------------

pub fn load_audio_presets() -> Option<Vec<AudioPreset>> {
    let path = audio_presets_path();
    match read_json::<Vec<AudioPreset>>(&path) {
        Ok(list) => {
            info!("loaded {} audio preset(s) from {}", list.len(), path.display());
            Some(list)
        }
        Err(e) => {
            info!("no audio presets file yet ({e})");
            None
        }
    }
}

pub fn save_audio_presets(presets: &[AudioPreset]) -> Result<(), String> {
    write_json(&audio_presets_path(), &presets)
}

pub fn load_preset_rules() -> Option<Vec<PresetRule>> {
    let path = preset_rules_path();
    match read_json::<Vec<PresetRule>>(&path) {
        Ok(list) => {
            info!("loaded {} preset rule(s) from {}", list.len(), path.display());
            Some(list)
        }
        Err(e) => {
            info!("no preset rules file yet ({e})");
            None
        }
    }
}

pub fn save_preset_rules(rules: &[PresetRule]) -> Result<(), String> {
    write_json(&preset_rules_path(), &rules)
}

// ---------------------------------------------------------------------------
// JSON helpers
// ---------------------------------------------------------------------------

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, String> {
    let data = fs::read_to_string(path).map_err(|e| e.to_string())?;
    serde_json::from_str(&data).map_err(|e| e.to_string())
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
