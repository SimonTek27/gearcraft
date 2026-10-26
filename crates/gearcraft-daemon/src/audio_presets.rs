//! Audio preset store (JDSP4Linux-compatible workflow).
//!
//! Persists named EQ/effects presets and device→preset rules under
//! `~/.config/gearcraft/audio_presets.json` and `preset_rules.json`.
//!
//! ChatMix is applied immediately; EQ/crossfeed go through the PipeWire
//! filter-chain bridge (`dsp_pw`) so GearCraft can replace JDSP4Linux.

use std::collections::HashMap;

use gearcraft_core::{AudioEffects, AudioPreset, ChatMixState, EqBand, PresetRule};
use tracing::{info, warn};

use crate::config;

#[derive(Default)]
pub struct AudioPresetStore {
    presets: HashMap<String, AudioPreset>,
    rules: Vec<PresetRule>,
}

impl AudioPresetStore {
    pub fn load() -> Self {
        let mut s = Self::default();
        if let Some(list) = config::load_audio_presets() {
            for p in list {
                s.presets.insert(p.id.clone(), p);
            }
        }
        if s.presets.is_empty() {
            for p in Self::builtin_presets() {
                s.presets.insert(p.id.clone(), p);
            }
            let _ = s.persist_presets();
        }
        s.rules = config::load_preset_rules().unwrap_or_default();
        info!(
            "audio presets: {} preset(s), {} rule(s)",
            s.presets.len(),
            s.rules.len()
        );
        s
    }

    fn builtin_presets() -> Vec<AudioPreset> {
        vec![
            AudioPreset {
                id: "flat".into(),
                name: "Flat".into(),
                effects: AudioEffects::default(),
                chatmix: Some(ChatMixState::default()),
                notes: "Neutral reference".into(),
            },
            AudioPreset {
                id: "bass-boost".into(),
                name: "Bass Boost".into(),
                effects: AudioEffects {
                    eq_enabled: true,
                    eq_bands: bass_shaped_eq(4.0),
                    bass_boost: true,
                    bass_boost_db: 6.0,
                    limiter: true,
                    ..AudioEffects::default()
                },
                chatmix: None,
                notes: "Low-shelf emphasis for consumer headphones".into(),
            },
            AudioPreset {
                id: "vocal".into(),
                name: "Vocal Clarity".into(),
                effects: AudioEffects {
                    eq_enabled: true,
                    eq_bands: vocal_eq(),
                    compressor: true,
                    limiter: true,
                    ..AudioEffects::default()
                },
                chatmix: None,
                notes: "Presence band lift for chat / podcasts".into(),
            },
            AudioPreset {
                id: "apex-legends".into(),
                name: "Apex Legends".into(),
                effects: AudioEffects {
                    eq_enabled: true,
                    eq_bands: vec![
                        gearcraft_core::EqBand { freq_hz: 80.0, gain_db: -1.5 },
                        gearcraft_core::EqBand { freq_hz: 250.0, gain_db: -2.0 },
                        gearcraft_core::EqBand { freq_hz: 1200.0, gain_db: 1.5 },
                        gearcraft_core::EqBand { freq_hz: 2800.0, gain_db: 3.5 },
                        gearcraft_core::EqBand { freq_hz: 6000.0, gain_db: 1.0 },
                    ],
                    spatial_enabled: true,
                    spatial_immersion: 15,
                    spatial_distance: 25,
                    volume_boost_db: 1.0,
                    smart_volume: true,
                    limiter: true,
                    ..AudioEffects::default()
                },
                chatmix: None,
                notes: "Footsteps / gunfire presence (performance spatial)".into(),
            },
            AudioPreset {
                id: "cs2".into(),
                name: "Counter-Strike 2".into(),
                effects: AudioEffects {
                    eq_enabled: true,
                    eq_bands: vec![
                        gearcraft_core::EqBand { freq_hz: 100.0, gain_db: -2.0 },
                        gearcraft_core::EqBand { freq_hz: 3000.0, gain_db: 4.0 },
                        gearcraft_core::EqBand { freq_hz: 8000.0, gain_db: 1.5 },
                    ],
                    spatial_enabled: true,
                    spatial_immersion: 10,
                    spatial_distance: 20,
                    volume_boost_db: 0.5,
                    smart_volume: true,
                    limiter: true,
                    ..AudioEffects::default()
                },
                chatmix: None,
                notes: "Competitive FPS — directional cues".into(),
            },
            AudioPreset {
                id: "immersion".into(),
                name: "Immersion / Story".into(),
                effects: AudioEffects {
                    eq_enabled: true,
                    eq_bands: vec![
                        gearcraft_core::EqBand { freq_hz: 80.0, gain_db: 3.0 },
                        gearcraft_core::EqBand { freq_hz: 400.0, gain_db: 0.5 },
                        gearcraft_core::EqBand { freq_hz: 4000.0, gain_db: 1.5 },
                        gearcraft_core::EqBand { freq_hz: 10000.0, gain_db: 2.0 },
                    ],
                    spatial_enabled: true,
                    spatial_immersion: 85,
                    spatial_distance: 60,
                    crossfeed: true,
                    crossfeed_strength: 25,
                    volume_boost_db: 0.0,
                    limiter: true,
                    ..AudioEffects::default()
                },
                chatmix: None,
                notes: "Single-player / cinematic spatial".into(),
            },
            AudioPreset {
                id: "crossfeed".into(),
                name: "Crossfeed".into(),
                effects: AudioEffects {
                    crossfeed: true,
                    crossfeed_strength: 40,
                    limiter: true,
                    ..AudioEffects::default()
                },
                chatmix: None,
                notes: "BS2B-style stereo image for headphones".into(),
            },
            AudioPreset {
                id: "game-fps".into(),
                name: "Game FPS".into(),
                effects: AudioEffects {
                    eq_enabled: true,
                    eq_bands: fps_eq(),
                    wideness: true,
                    wideness_pct: 25,
                    limiter: true,
                    ..AudioEffects::default()
                },
                chatmix: Some(ChatMixState {
                    balance: 20,
                    game_volume: 100,
                    chat_volume: 80,
                }),
                notes: "Footsteps / positional cues + slight game bias".into(),
            },
        ]
    }

    pub fn list(&self) -> Vec<AudioPreset> {
        let mut v: Vec<_> = self.presets.values().cloned().collect();
        v.sort_by(|a, b| a.name.cmp(&b.name));
        v
    }

    pub fn get(&self, id: &str) -> Option<&AudioPreset> {
        self.presets.get(id)
    }

    pub fn save(&mut self, preset: AudioPreset) -> Result<(), String> {
        if preset.id.is_empty() || preset.name.is_empty() {
            return Err("preset id and name required".into());
        }
        self.presets.insert(preset.id.clone(), preset);
        self.persist_presets()
    }

    pub fn delete(&mut self, id: &str) -> Result<bool, String> {
        let removed = self.presets.remove(id).is_some();
        if removed {
            self.rules.retain(|r| r.preset_id != id);
            self.persist_presets()?;
            self.persist_rules()?;
        }
        Ok(removed)
    }

    pub fn list_rules(&self) -> Vec<PresetRule> {
        self.rules.clone()
    }

    pub fn set_rule(&mut self, rule: PresetRule) -> Result<(), String> {
        if rule.device_id.is_empty() || rule.preset_id.is_empty() {
            return Err("device_id and preset_id required".into());
        }
        if !self.presets.contains_key(&rule.preset_id) {
            return Err(format!("unknown preset: {}", rule.preset_id));
        }
        self.rules
            .retain(|r| !(r.device_id == rule.device_id && r.route_id == rule.route_id));
        self.rules.push(rule);
        self.persist_rules()
    }

    pub fn delete_rule(&mut self, device_id: &str, route_id: &str) -> Result<bool, String> {
        let before = self.rules.len();
        self.rules
            .retain(|r| !(r.device_id == device_id && r.route_id == route_id));
        let removed = self.rules.len() < before;
        if removed {
            self.persist_rules()?;
        }
        Ok(removed)
    }

    /// Resolve which preset should apply for a device (exact route, then any route).
    pub fn rule_for(&self, device_id: &str, route_id: &str) -> Option<&AudioPreset> {
        let pid = self
            .rules
            .iter()
            .find(|r| r.device_id == device_id && r.route_id == route_id)
            .or_else(|| {
                self.rules
                    .iter()
                    .find(|r| r.device_id == device_id && r.route_id.is_empty())
            })
            .map(|r| r.preset_id.as_str())?;
        self.presets.get(pid)
    }

    fn persist_presets(&self) -> Result<(), String> {
        let list = self.list();
        config::save_audio_presets(&list)?;
        info!("audio presets saved ({} entries)", list.len());
        Ok(())
    }

    fn persist_rules(&self) -> Result<(), String> {
        config::save_preset_rules(&self.rules)?;
        info!("preset rules saved ({} entries)", self.rules.len());
        Ok(())
    }
}

fn bass_shaped_eq(boost: f32) -> Vec<EqBand> {
    let freqs = [
        31.5, 63.0, 125.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0, 16000.0,
    ];
    let gains = [
        boost, boost * 0.85, boost * 0.5, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
    ];
    freqs
        .iter()
        .zip(gains)
        .map(|(&freq_hz, gain_db)| EqBand { freq_hz, gain_db })
        .collect()
}

fn vocal_eq() -> Vec<EqBand> {
    let freqs = [
        31.5, 63.0, 125.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0, 16000.0,
    ];
    let gains = [-2.0, -1.0, 0.0, 1.0, 2.0, 3.0, 2.5, 1.0, 0.0, -1.0];
    freqs
        .iter()
        .zip(gains)
        .map(|(&freq_hz, gain_db)| EqBand { freq_hz, gain_db })
        .collect()
}

fn fps_eq() -> Vec<EqBand> {
    let freqs = [
        31.5, 63.0, 125.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0, 16000.0,
    ];
    // Mild scoop + upper-mid for footsteps
    let gains = [1.0, 0.5, -1.0, -2.0, -1.0, 1.5, 3.0, 2.0, 1.0, 0.0];
    freqs
        .iter()
        .zip(gains)
        .map(|(&freq_hz, gain_db)| EqBand { freq_hz, gain_db })
        .collect()
}

/// Apply preset side-effects that the daemon can handle today (ChatMix).
pub async fn apply_preset_runtime(preset: &AudioPreset) -> Result<serde_json::Value, String> {
    let mut applied = serde_json::json!({
        "preset_id": preset.id,
        "preset_name": preset.name,
        "effects": preset.effects,
        "dsp_engine": "wireplumber-wpexec",
        "chatmix_applied": false,
        "dsp_applied": false,
    });

    if let Some(ref cm) = preset.chatmix {
        if let Err(e) = crate::config::save_chatmix(cm) {
            warn!("ChatMix persist on preset load: {e}");
        }
        match crate::chatmix_pw::apply_volumes(cm.game_volume, cm.chat_volume).await {
            Ok(_) => {
                applied["chatmix_applied"] = serde_json::json!(true);
                applied["chatmix"] = serde_json::to_value(cm).unwrap_or_default();
            }
            Err(e) => {
                applied["chatmix_message"] = serde_json::json!(e);
            }
        }
    }

    match crate::dsp_pw::apply_effects(&preset.effects).await {
        Ok(dsp) => {
            applied["dsp_applied"] = serde_json::json!(dsp.applied);
            applied["dsp_eq"] = serde_json::json!(dsp.eq);
            applied["dsp_crossfeed"] = serde_json::json!(dsp.crossfeed);
            applied["dsp_conf"] = serde_json::json!(dsp.conf_path);
            applied["dsp_message"] = serde_json::json!(dsp.message);
        }
        Err(e) => {
            warn!("PipeWire DSP apply failed: {e}");
            applied["dsp_message"] = serde_json::json!(e);
        }
    }

    info!(
        "loaded audio preset '{}' (eq={} bands, bass={} crossfeed={})",
        preset.name,
        preset.effects.eq_bands.len(),
        preset.effects.bass_boost,
        preset.effects.crossfeed
    );
    Ok(applied)
}
