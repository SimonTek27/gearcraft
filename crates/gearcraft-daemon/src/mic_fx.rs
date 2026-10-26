//! Microphone FX — real high-pass + noise gate via WirePlumber filter-chain.
//!
//! Virtual source `gearcraft.mic` ("GearCraft Mic"):
//!   hardware capture → [bq_highpass] → [LADSPA gate / fallback] → source for apps
//!
//! Gain/mute applied with pactl on the virtual source when present.

use std::fs;
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;

use gearcraft_core::MicFxState;
use tokio::process::Command;
use tracing::{info, warn};

const NODE_NAME: &str = "gearcraft.mic";
const SOURCE_DESC: &str = "GearCraft Mic";

#[derive(Debug, Clone)]
pub struct MicApplyResult {
    pub applied: bool,
    pub source: String,
    pub message: String,
}

fn dsp_dir() -> PathBuf {
    let d = crate::config::config_dir().join("dsp");
    let _ = fs::create_dir_all(&d);
    d
}

fn mic_lua_path() -> PathBuf {
    dsp_dir().join("gearcraft-mic.lua")
}

fn mic_json_path() -> PathBuf {
    dsp_dir().join("mic-filter-chain.json")
}

fn mic_pid_path() -> PathBuf {
    dsp_dir().join("wpexec-mic.pid")
}

async fn wpexec_available() -> bool {
    Command::new("which")
        .arg("wpexec")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await
        .map(|s| s.success())
        .unwrap_or(false)
}

async fn physical_default_source() -> Option<String> {
    let out = Command::new("pactl")
        .args(["get-default-source"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .await
        .ok()?;
    if !out.status.success() {
        return first_hardware_source().await;
    }
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if s.is_empty() || s.contains("gearcraft") {
        return first_hardware_source().await;
    }
    Some(s)
}

async fn first_hardware_source() -> Option<String> {
    let out = Command::new("pactl")
        .args(["list", "short", "sources"])
        .output()
        .await
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    for line in text.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 2 {
            continue;
        }
        let name = parts[1];
        if name.contains("gearcraft") || name.contains("monitor") {
            continue;
        }
        return Some(name.to_string());
    }
    None
}

/// Returns (plugin_path, label)
fn find_ladspa_gate() -> Option<(String, String)> {
    const CANDIDATES: &[(&str, &str)] = &[
        ("/usr/lib/ladspa/gate_1410.so", "gate"),
        ("/usr/lib64/ladspa/gate_1410.so", "gate"),
        ("/usr/lib/x86_64-linux-gnu/ladspa/gate_1410.so", "gate"),
        ("/usr/lib/ladspa/sc4_1882.so", "sc4"),
        ("/usr/lib64/ladspa/sc4_1882.so", "sc4"),
        ("/usr/lib/x86_64-linux-gnu/ladspa/sc4_1882.so", "sc4"),
    ];
    for (path, label) in CANDIDATES {
        if PathBuf::from(path).exists() {
            return Some((path.to_string(), label.to_string()));
        }
    }
    None
}

struct MicGraph {
    json: String,
    gate_note: String,
}

fn build_mic_filter_json(state: &MicFxState, target_source: &str) -> MicGraph {
    let mut nodes: Vec<String> = Vec::new();
    let mut links: Vec<String> = Vec::new();
    let mut prev: Option<String> = None;
    let mut gate_note = String::from("gate off");

    fn push_hp(nodes: &mut Vec<String>, links: &mut Vec<String>, prev: &mut Option<String>, name: &str, freq: f32) {
        nodes.push(format!(
            r#"{{"type":"builtin","name":"{name}","label":"bq_highpass","control":{{"Freq":{freq:.2},"Q":0.707}}}}"#
        ));
        if let Some(ref p) = *prev {
            links.push(format!(
                r#"{{"output":"{p}:Out","input":"{name}:In"}}"#
            ));
        }
        *prev = Some(name.to_string());
    }

    if state.highpass {
        let hz = state.highpass_hz.clamp(40.0, 300.0);
        push_hp(&mut nodes, &mut links, &mut prev, "gc_mic_hp", hz);
    }

    if state.lowpass {
        let hz = state.lowpass_hz.clamp(3000.0, 16000.0);
        nodes.push(format!(
            r#"{{"type":"builtin","name":"gc_mic_lp","label":"bq_lowpass","control":{{"Freq":{hz:.1},"Q":0.707}}}}"#
        ));
        if let Some(ref p) = prev {
            links.push(format!(
                r#"{{"output":"{p}:Out","input":"gc_mic_lp:In"}}"#
            ));
        }
        prev = Some("gc_mic_lp".to_string());
    }

    if state.noise_gate {
        let strength = (state.gate_strength as f32 / 100.0).clamp(0.0, 1.0);
        if let Some((plugin, label)) = find_ladspa_gate() {
            let name = "gc_mic_gate";
            if label == "sc4" {
                let threshold = -40.0 + strength * 25.0;
                let ratio = 2.0 + strength * 6.0;
                nodes.push(format!(
                    r#"{{"type":"ladspa","name":"{name}","plugin":"{plugin}","label":"sc4","control":{{"RMS/peak":0,"Attack time (ms)":5,"Release time (ms)":50,"Threshold level (dB)":{threshold:.1},"Ratio (1:n)":{ratio:.1},"Knee radius (dB)":3,"Makeup gain (dB)":0}}}}"#
                ));
                gate_note = format!("LADSPA sc4 compressor/gate (threshold {threshold:.0} dB)");
            } else {
                let threshold = -50.0 + strength * 35.0;
                nodes.push(format!(
                    r#"{{"type":"ladspa","name":"{name}","plugin":"{plugin}","label":"{label}","control":{{"Threshold":{threshold:.1},"Attack":5,"Hold":50,"Decay":100,"Range":-60,"Output":0}}}}"#
                ));
                gate_note = format!("LADSPA {label} (threshold {threshold:.0} dB)");
            }
            if let Some(ref p) = prev {
                links.push(format!(
                    r#"{{"output":"{p}:Out","input":"{name}:In"}}"#
                ));
            }
            prev = Some(name.to_string());
            let _ = prev;
        } else {
            // No LADSPA gate: reinforce high-pass + mild low shelf cut
            if !state.highpass {
                push_hp(&mut nodes, &mut links, &mut prev, "gc_mic_hp_gate", 120.0);
            }
            nodes.push(
                r#"{"type":"builtin","name":"gc_mic_gate_soft","label":"bq_lowshelf","control":{"Freq":200.0,"Q":0.7,"Gain":-4.0}}"#
                    .into(),
            );
            if let Some(ref p) = prev {
                links.push(format!(
                    r#"{{"output":"{p}:Out","input":"gc_mic_gate_soft:In"}}"#
                ));
            }
            gate_note = "soft fallback (install swh-plugins for gate_1410)".into();
        }
    }

    if state.compressor {
        let strength = (state.compressor_strength as f32 / 100.0).clamp(0.05, 1.0);
        if let Some((plugin, label)) = find_ladspa_gate() {
            // Prefer sc4; if only gate found, still use sc4 path when label is sc4
            let name = "gc_mic_comp";
            let threshold = -28.0 + (1.0 - strength) * 14.0;
            let ratio = 2.5 + strength * 5.0;
            let makeup = strength * 4.0;
            if label == "sc4" || plugin.contains("sc4") {
                nodes.push(format!(
                    r#"{{"type":"ladspa","name":"{name}","plugin":"{plugin}","label":"sc4","control":{{"RMS/peak":0,"Attack time (ms)":8,"Release time (ms)":80,"Threshold level (dB)":{threshold:.1},"Ratio (1:n)":{ratio:.1},"Knee radius (dB)":4,"Makeup gain (dB)":{makeup:.1}}}}}"#
                ));
                gate_note = format!("{gate_note}; compressor sc4");
            } else if let Some(sc4) = [
                "/usr/lib/ladspa/sc4_1882.so",
                "/usr/lib64/ladspa/sc4_1882.so",
                "/usr/lib/x86_64-linux-gnu/ladspa/sc4_1882.so",
            ].iter().find(|p| PathBuf::from(p).exists()) {
                nodes.push(format!(
                    r#"{{"type":"ladspa","name":"{name}","plugin":"{sc4}","label":"sc4","control":{{"RMS/peak":0,"Attack time (ms)":8,"Release time (ms)":80,"Threshold level (dB)":{threshold:.1},"Ratio (1:n)":{ratio:.1},"Knee radius (dB)":4,"Makeup gain (dB)":{makeup:.1}}}}}"#
                ));
                gate_note = format!("{gate_note}; compressor sc4");
            } else {
                nodes.push(format!(
                    r#"{{"type":"builtin","name":"{name}","label":"bq_peaking","control":{{"Freq":1800.0,"Q":0.9,"Gain":{g:.2}}}}}"#,
                    g = -2.0 * strength,
                ));
                gate_note = format!("{gate_note}; compressor soft fallback");
            }
            if let Some(ref p) = prev {
                links.push(format!(
                    r#"{{"output":"{p}:Out","input":"{name}:In"}}"#
                ));
            }
            prev = Some(name.to_string());
            let _ = prev;
        } else {
            let name = "gc_mic_comp";
            let g = -2.0 * strength;
            nodes.push(format!(
                r#"{{"type":"builtin","name":"{name}","label":"bq_peaking","control":{{"Freq":1800.0,"Q":0.9,"Gain":{g:.2}}}}}"#
            ));
            if let Some(ref p) = prev {
                links.push(format!(
                    r#"{{"output":"{p}:Out","input":"{name}:In"}}"#
                ));
            }
            gate_note = format!("{gate_note}; compressor soft fallback (install swh-plugins)");
        }
    }

    if nodes.is_empty() {
        nodes.push(r#"{"type":"builtin","name":"gc_mic_thru","label":"copy"}"#.into());
    }

    let json = format!(
        r#"{{
  "node.description": "{SOURCE_DESC}",
  "media.name": "{SOURCE_DESC}",
  "audio.channels": 1,
  "audio.position": ["MONO"],
  "filter.graph": {{
    "nodes": [
      {nodes}
    ],
    "links": [
      {links}
    ]
  }},
  "capture.props": {{
    "node.name": "{NODE_NAME}.input",
    "media.class": "Audio/Sink",
    "audio.channels": 1,
    "filter.smart": true,
    "filter.smart.name": "gearcraft-mic",
    "filter.smart.target": "{target}"
  }},
  "playback.props": {{
    "node.name": "{NODE_NAME}",
    "media.class": "Audio/Source",
    "node.description": "{SOURCE_DESC}",
    "audio.channels": 1
  }}
}}"#,
        nodes = nodes.join(",\n      "),
        links = links.join(",\n      "),
        target = target_source,
    );

    MicGraph { json, gate_note }
}

fn build_wpexec_lua(args_json: &str) -> String {
    // Escape for Lua long string
    let escaped = args_json.replace("]=]", "] =]");
    format!(
        r#"#!/usr/bin/env wpexec
-- GearCraft mic filter-chain (high-pass + gate)
local args = [=[
{escaped}
]=]

local m = LocalModule("libpipewire-module-filter-chain", args, {{}})
assert(m, "failed to load mic filter-chain")
Core.quit = function() end
"#
    )
}

async fn stop_mic_wpexec() {
    let path = mic_pid_path();
    if let Ok(s) = fs::read_to_string(&path) {
        if let Ok(pid) = s.trim().parse::<i32>() {
            if pid > 1 {
                let _ = Command::new("kill")
                    .args(["-TERM", &pid.to_string()])
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status()
                    .await;
                tokio::time::sleep(Duration::from_millis(150)).await;
                let _ = Command::new("kill")
                    .args(["-KILL", &pid.to_string()])
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status()
                    .await;
                info!("stopped previous mic wpexec pid={pid}");
            }
        }
    }
    let _ = fs::remove_file(&path);
    let _ = Command::new("pkill")
        .args(["-f", "gearcraft-mic.lua"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await;
}

async fn set_source_mute(source: &str, muted: bool) -> Result<(), String> {
    let arg = if muted { "1" } else { "0" };
    let st = Command::new("pactl")
        .args(["set-source-mute", source, arg])
        .status()
        .await
        .map_err(|e| e.to_string())?;
    if st.success() {
        Ok(())
    } else {
        Err("pactl set-source-mute failed".into())
    }
}

async fn set_source_volume(source: &str, pct: u8) -> Result<(), String> {
    let pct = pct.min(150);
    let st = Command::new("pactl")
        .args(["set-source-volume", source, &format!("{pct}%")])
        .status()
        .await
        .map_err(|e| e.to_string())?;
    if st.success() {
        Ok(())
    } else {
        Err("pactl set-source-volume failed".into())
    }
}

async fn try_set_default_source(name: &str) {
    let _ = Command::new("pactl")
        .args(["set-default-source", name])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await;
}

/// Apply real high-pass / gate DSP + gain/mute.
pub async fn apply_mic_fx(state: &MicFxState) -> Result<MicApplyResult, String> {
    let physical = physical_default_source()
        .await
        .ok_or_else(|| "no hardware mic source found".to_string())?;

    let need_dsp = state.enabled && (state.highpass || state.noise_gate || state.lowpass || state.compressor || state.rnnoise);

    if !state.enabled {
        stop_mic_wpexec().await;
        let _ = set_source_mute(&physical, false).await;
        return Ok(MicApplyResult {
            applied: true,
            source: physical.clone(),
            message: format!("Mic FX off — using {physical}"),
        });
    }

    let mut message_parts: Vec<String> = Vec::new();
    let mut active_source = physical.clone();

    if need_dsp {
        if !wpexec_available().await {
            warn!("wpexec missing — mic DSP skipped, gain/mute only");
            message_parts.push("DSP skipped (install wireplumber/wpexec)".into());
        } else {
            let graph = build_mic_filter_json(state, &physical);
            fs::write(mic_json_path(), &graph.json)
                .map_err(|e| format!("write mic json: {e}"))?;
            let lua = build_wpexec_lua(&graph.json);
            let lua_path = mic_lua_path();
            fs::write(&lua_path, &lua).map_err(|e| format!("write mic lua: {e}"))?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = fs::set_permissions(&lua_path, fs::Permissions::from_mode(0o755));
            }

            stop_mic_wpexec().await;

            let child = Command::new("wpexec")
                .arg(&lua_path)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .kill_on_drop(false)
                .spawn()
                .map_err(|e| format!("spawn wpexec mic: {e}"))?;
            let pid = child.id().ok_or_else(|| "mic wpexec pid unavailable".to_string())?;
            fs::write(mic_pid_path(), pid.to_string()).map_err(|e| format!("write mic pid: {e}"))?;
            std::mem::forget(child);

            tokio::time::sleep(Duration::from_millis(400)).await;

            // Prefer virtual source for apps
            active_source = NODE_NAME.to_string();
            try_set_default_source(NODE_NAME).await;

            if state.highpass {
                message_parts.push(format!(
                    "high-pass {:.0} Hz (bq_highpass)",
                    state.highpass_hz.clamp(40.0, 300.0)
                ));
            }
            if state.lowpass {
                message_parts.push(format!(
                    "low-pass {:.0} Hz (bq_lowpass)",
                    state.lowpass_hz.clamp(3000.0, 16000.0)
                ));
            }
            if state.noise_gate || state.compressor {
                message_parts.push(graph.gate_note);
            }
            message_parts.push(format!("wpexec pid {pid}"));
            message_parts.push(format!("source «{SOURCE_DESC}»"));
        }
    } else {
        stop_mic_wpexec().await;
        message_parts.push("no DSP filters (gain/mute only)".into());
    }

    // Gain / mute on active source
    if set_source_mute(&active_source, state.muted).await.is_err() {
        let _ = set_source_mute(&physical, state.muted).await;
    }
    if !state.muted {
        let _ = set_source_volume(&active_source, state.gain_pct).await;
        let _ = set_source_volume(&physical, state.gain_pct).await;
    }
    message_parts.push(format!("gain {}%", state.gain_pct));
    if state.muted {
        message_parts.push("muted".into());
    }

    
    if state.rnnoise && state.enabled {
        // Prefer PipeWire RNNoise LADSPA / LV2 if present; best-effort load
        let rn_paths = [
            "/usr/lib/ladspa/librnnoise_ladspa.so",
            "/usr/lib64/ladspa/librnnoise_ladspa.so",
            "/usr/lib/x86_64-linux-gnu/ladspa/librnnoise_ladspa.so",
        ];
        let plugin = rn_paths.iter().find(|p| PathBuf::from(p).exists());
        if plugin.is_some() {
            message_parts.push("rnnoise plugin present (enable via filter-chain/EasyEffects if not active)".into());
        } else {
            // Try PipeWire echo-cancel as mild noise suppress fallback
            let _ = Command::new("pactl")
                .args([
                    "load-module",
                    "module-echo-cancel",
                    "use_master_format=1",
                    "aec_args=analog_gain_control=0 digital_gain_control=1",
                ])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .await;
            message_parts.push(
                "rnnoise: install noise-suppression-for-voice / rnnoise ladspa; attempted echo-cancel"
                    .into(),
            );
        }
    }

    let message = format!("Mic FX: {}", message_parts.join("; "));
    info!("{message}");
    Ok(MicApplyResult {
        applied: true,
        source: active_source,
        message,
    })
}

pub async fn read_mic_snapshot() -> MicFxState {
    let mut s = MicFxState::default();
    let source = physical_default_source().await;
    let src = source.as_deref().unwrap_or(NODE_NAME);

    if let Ok(out) = Command::new("pactl")
        .args(["get-source-volume", src])
        .output()
        .await
    {
        if out.status.success() {
            let text = String::from_utf8_lossy(&out.stdout);
            if let Some(idx) = text.find('%') {
                let head = &text[..idx];
                if let Some(num) = head
                    .rsplit(|c: char| !c.is_ascii_digit())
                    .find(|x| !x.is_empty())
                {
                    if let Ok(v) = num.parse::<u16>() {
                        s.gain_pct = v.min(150) as u8;
                    }
                }
            }
        }
    }
    if let Ok(out) = Command::new("pactl")
        .args(["get-source-mute", src])
        .output()
        .await
    {
        if out.status.success() {
            let text = String::from_utf8_lossy(&out.stdout);
            s.muted = text.contains("yes");
        }
    }
    s
}
