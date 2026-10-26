//! Import JamesDSP / JDSP4Linux preset `.conf` files into GearCraft `AudioPreset`.
//!
//! Format is key=value (see ~/.config/jamesdsp/presets/*.conf).
//! Unsupported blocks (convolver IR, DDC, liveprog, reverb room model) are
//! recorded in notes; mapped effects are applied through our WirePlumber DSP.

use std::collections::HashMap;
use std::path::Path;

use gearcraft_core::{AudioEffects, AudioPreset, EqBand};
use tracing::info;

/// Parse a JamesDSP `.conf` string into key → value.
pub fn parse_conf(text: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        if let Some((k, v)) = line.split_once('=') {
            map.insert(k.trim().to_string(), v.trim().to_string());
        }
    }
    map
}

fn bool_val(map: &HashMap<String, String>, key: &str) -> bool {
    map.get(key)
        .map(|v| matches!(v.to_ascii_lowercase().as_str(), "true" | "1" | "yes"))
        .unwrap_or(false)
}

fn f32_val(map: &HashMap<String, String>, key: &str, default: f32) -> f32 {
    map.get(key)
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

fn u8_val(map: &HashMap<String, String>, key: &str, default: u8) -> u8 {
    map.get(key)
        .and_then(|v| v.parse::<f32>().ok())
        .map(|v| v.clamp(0.0, 100.0) as u8)
        .unwrap_or(default)
}

/// JamesDSP `tone_eq` is: freq1;freq2;...;freqN;gain1;gain2;...;gainN
/// (N frequencies then N gains). Also accept GraphicEQ style from graphiceq_param.
fn parse_tone_eq(map: &HashMap<String, String>) -> Vec<EqBand> {
    if let Some(raw) = map.get("tone_eq") {
        let parts: Vec<f32> = raw
            .split(';')
            .filter_map(|s| s.trim().parse().ok())
            .collect();
        if parts.len() >= 2 && parts.len() % 2 == 0 {
            let n = parts.len() / 2;
            let mut bands = Vec::with_capacity(n);
            for i in 0..n {
                bands.push(EqBand {
                    freq_hz: parts[i],
                    gain_db: parts[n + i].clamp(-12.0, 12.0),
                });
            }
            return bands;
        }
    }
    // GraphicEQ: 25 0; 40 0; ... or "GraphicEQ: 25 0; 40 1"
    if let Some(raw) = map.get("graphiceq_param") {
        let body = raw
            .strip_prefix("GraphicEQ:")
            .unwrap_or(raw)
            .trim();
        let mut bands = Vec::new();
        for seg in body.split(';') {
            let mut it = seg.split_whitespace();
            if let (Some(f), Some(g)) = (it.next(), it.next()) {
                if let (Ok(freq_hz), Ok(gain_db)) = (f.parse::<f32>(), g.parse::<f32>()) {
                    bands.push(EqBand {
                        freq_hz,
                        gain_db: gain_db.clamp(-12.0, 12.0),
                    });
                }
            }
        }
        if !bands.is_empty() {
            return bands;
        }
    }
    Vec::new()
}

fn slug_id(name: &str) -> String {
    let s: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    let s = s.trim_matches('-').to_string();
    if s.is_empty() {
        "jdsp-import".into()
    } else {
        format!("jdsp-{s}")
    }
}

/// Convert parsed JamesDSP conf into a GearCraft preset.
pub fn conf_to_preset(name: &str, map: &HashMap<String, String>) -> AudioPreset {
    let mut fx = AudioEffects::default();
    let mut notes: Vec<String> = vec!["Imported from JamesDSP .conf".into()];

    // Tone / multimodal EQ
    let bands = parse_tone_eq(map);
    let tone_on = bool_val(map, "tone_enable") || bool_val(map, "graphiceq_enable");
    if !bands.is_empty() {
        fx.eq_enabled = tone_on || bands.iter().any(|b| b.gain_db.abs() > 0.05);
        fx.eq_bands = bands;
    } else {
        fx.eq_enabled = tone_on;
    }

    // Bass
    fx.bass_boost = bool_val(map, "bass_enable");
    fx.bass_boost_db = f32_val(map, "bass_maxgain", 0.0).clamp(0.0, 12.0);

    // Crossfeed (BS2B feed 0–150 style → 0–100)
    fx.crossfeed = bool_val(map, "crossfeed_enable");
    let feed = f32_val(map, "crossfeed_bs2b_feed", 60.0);
    fx.crossfeed_strength = (feed / 1.5).clamp(0.0, 100.0) as u8;

    // Compander → compressor approximation
    fx.compressor = bool_val(map, "compander_enable");
    fx.compressor_strength = if fx.compressor { 50 } else { 0 };

    // Stereo wideness
    fx.wideness = bool_val(map, "stereowide_enable");
    fx.wideness_pct = u8_val(map, "stereowide_level", 0);

    // Limiter
    fx.limiter = true;
    fx.limiter_db = f32_val(map, "master_limthreshold", -0.1);

    // Post-gain as volume boost
    fx.volume_boost_db = f32_val(map, "master_postgain", 0.0).clamp(-12.0, 12.0);

    // Bypass when master disabled
    fx.bypass = !bool_val(map, "master_enable") && map.contains_key("master_enable");

    // Unsupported / partial → notes (detailed mapping: docs/JAMESDSP_MAPPING.md)
    if bool_val(map, "convolver_enable") {
        let f = map.get("convolver_file").map(|s| s.as_str()).unwrap_or("?");
        notes.push(format!("convolver IR not mapped (file={f})"));
    }
    if bool_val(map, "ddc_enable") {
        let f = map.get("ddc_file").map(|s| s.as_str()).unwrap_or("?");
        notes.push(format!("ViPER-DDC not mapped (file={f})"));
    }
    if bool_val(map, "reverb_enable") {
        fx.reverb = true;
        let decay = f32_val(map, "reverb_decay", 0.0);
        notes.push(format!(
            "reverb Progenitor not implemented (decay={decay}); flag only"
        ));
    }
    if bool_val(map, "liveprog_enable") {
        let f = map.get("liveprog_file").map(|s| s.as_str()).unwrap_or("?");
        notes.push(format!("liveprog/EEL2 not executed (file={f})"));
    }
    if bool_val(map, "tube_enable") {
        let g = f32_val(map, "tube_pregain", 0.0);
        notes.push(format!("tube/analog not mapped (pregain={g})"));
    }
    if bool_val(map, "bass_enable") {
        notes.push("dynamic bass → static low-shelf (no auto bandwidth)".into());
    }
    if bool_val(map, "compander_enable") {
        notes.push("multiband compander → single-band sc4 compressor".into());
    }
    // Crossfeed cut-off note
    if fx.crossfeed {
        let fcut = f32_val(map, "crossfeed_bs2b_fcut", 700.0);
        notes.push(format!("crossfeed fcut={fcut} Hz (plugin default may differ)"));
    }

    AudioPreset {
        id: slug_id(name),
        name: name.to_string(),
        effects: fx,
        chatmix: None,
        notes: notes.join("; "),
    }
}

pub fn import_file(path: &Path) -> Result<AudioPreset, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    let map = parse_conf(&text);
    if map.is_empty() {
        return Err("empty or invalid JamesDSP conf".into());
    }
    let name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("JamesDSP Import");
    let preset = conf_to_preset(name, &map);
    info!(
        "imported JamesDSP preset «{}» (eq={} bands, bass={}, xf={})",
        preset.name,
        preset.effects.eq_bands.len(),
        preset.effects.bass_boost,
        preset.effects.crossfeed
    );
    Ok(preset)
}

pub fn import_string(name: &str, content: &str) -> Result<AudioPreset, String> {
    let map = parse_conf(content);
    if map.is_empty() {
        return Err("empty or invalid JamesDSP conf".into());
    }
    Ok(conf_to_preset(name, &map))
}

/// Import all `*.conf` under a directory (non-recursive).
pub fn import_dir(dir: &Path) -> Result<Vec<AudioPreset>, String> {
    let rd = std::fs::read_dir(dir).map_err(|e| format!("read_dir: {e}"))?;
    let mut out = Vec::new();
    for ent in rd.flatten() {
        let p = ent.path();
        if p.extension().and_then(|e| e.to_str()) == Some("conf") {
            match import_file(&p) {
                Ok(pr) => out.push(pr),
                Err(e) => tracing::warn!("skip {}: {e}", p.display()),
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_tone_eq_pairs() {
        let mut map = HashMap::new();
        map.insert(
            "tone_eq".into(),
            "25;40;63;100;1.0;2.0;3.0;4.0".into(),
        );
        map.insert("tone_enable".into(), "true".into());
        let p = conf_to_preset("Test", &map);
        assert!(p.effects.eq_enabled);
        assert_eq!(p.effects.eq_bands.len(), 4);
        assert!((p.effects.eq_bands[0].freq_hz - 25.0).abs() < 0.01);
        assert!((p.effects.eq_bands[0].gain_db - 1.0).abs() < 0.01);
    }
}
