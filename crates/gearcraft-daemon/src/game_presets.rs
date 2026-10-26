//! Quick-set style game → audio preset auto-switch.
//!
//! Watches running applications (PipeWire sink-inputs + `/proc`) and, when a
//! known game binary appears, applies the matching audio preset (EQ/spatial).
//! Sonar-equivalent of “Quick Set” / game configurations.

use std::collections::HashSet;
use std::path::Path;
use std::sync::Arc;

use tokio::process::Command;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::audio_presets::{self, AudioPresetStore};

/// (substring match on binary/name lowercase, preset_id)
const GAME_MAP: &[(&str, &str)] = &[
    // Apex
    ("r5apex", "apex-legends"),
    ("apex_legends", "apex-legends"),
    ("apexlegends", "apex-legends"),
    // CS2 / CSGO
    ("cs2", "cs2"),
    ("csgo", "cs2"),
    ("counter-strike", "cs2"),
    // Immersive / story-leaning defaults
    ("eldenring", "immersion"),
    ("elden ring", "immersion"),
    ("godofwar", "immersion"),
    ("cyberpunk", "immersion"),
    ("witcher3", "immersion"),
    ("hl2", "immersion"),
    ("re4", "immersion"),
    // Competitive-ish → CS2-style presence
    ("valorant", "cs2"),
    ("overwatch", "cs2"),
    ("ow2", "cs2"),
    ("fortnite", "cs2"),
    ("r5apex_dx12", "apex-legends"),
];

pub async fn run_loop(store: Arc<RwLock<AudioPresetStore>>, interval_secs: u64) {
    let mut last_preset: Option<String> = None;
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(interval_secs.max(2)));
    info!("game preset auto-switch active (every {interval_secs}s)");

    loop {
        interval.tick().await;
        let names = collect_app_names().await;
        if names.is_empty() {
            continue;
        }

        let mut matched: Option<(&str, &str)> = None;
        for name in &names {
            let lower = name.to_ascii_lowercase();
            for (key, preset_id) in GAME_MAP {
                if lower.contains(key) {
                    matched = Some((key, preset_id));
                    break;
                }
            }
            if matched.is_some() {
                break;
            }
        }

        let Some((key, preset_id)) = matched else {
            continue;
        };

        if last_preset.as_deref() == Some(preset_id) {
            continue;
        }

        let preset = {
            let s = store.read().await;
            s.get(preset_id).cloned()
        };

        match preset {
            Some(p) => {
                info!(
                    "game detected («{key}») → auto-apply preset «{}» ({preset_id})",
                    p.name
                );
                match audio_presets::apply_preset_runtime(&p).await {
                    Ok(_) => {
                        last_preset = Some(preset_id.to_string());
                        info!("game preset «{}» applied", p.name);
                    }
                    Err(e) => warn!("game preset apply failed: {e}"),
                }
            }
            None => {
                debug!("game map hit {preset_id} but preset missing from store");
            }
        }
    }
}

async fn collect_app_names() -> HashSet<String> {
    let mut out = HashSet::new();

    // 1) PipeWire/Pulse sink-input properties (apps actively playing audio)
    if let Ok(output) = Command::new("pactl")
        .args(["list", "sink-inputs"])
        .output()
        .await
    {
        if output.status.success() {
            let text = String::from_utf8_lossy(&output.stdout);
            for line in text.lines() {
                let t = line.trim();
                for prefix in [
                    "application.process.binary = ",
                    "application.name = ",
                    "application.process.binary = \"",
                ] {
                    if let Some(rest) = t.strip_prefix(prefix) {
                        let name = rest.trim().trim_matches('"').to_string();
                        if !name.is_empty() {
                            out.insert(name);
                        }
                    }
                }
                // pactl format: application.process.binary = "cs2"
                if t.starts_with("application.") && t.contains('=') {
                    if let Some((_, val)) = t.split_once('=') {
                        let name = val.trim().trim_matches('"').to_string();
                        if !name.is_empty() && name.len() < 128 {
                            out.insert(name);
                        }
                    }
                }
            }
        }
    }

    // 2) /proc process names (catches games not yet routed to a sink)
    if let Ok(entries) = std::fs::read_dir("/proc") {
        for ent in entries.flatten() {
            let name = ent.file_name();
            let s = name.to_string_lossy();
            if !s.chars().all(|c| c.is_ascii_digit()) {
                continue;
            }
            let comm = Path::new("/proc").join(&*s).join("comm");
            if let Ok(c) = std::fs::read_to_string(comm) {
                let c = c.trim().to_string();
                if !c.is_empty() {
                    out.insert(c);
                }
            }
        }
    }

    out
}
