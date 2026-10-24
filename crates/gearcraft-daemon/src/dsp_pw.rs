//! WirePlumber DSP bridge — EQ + crossfeed via `wpexec` + filter-chain.
//!
//! Loads `libpipewire-module-filter-chain` dynamically through WirePlumber's
//! Lua runtime (`wpexec`), so presets apply without restarting PipeWire.
//!
//! Sink node: `gearcraft.dsp` ("GearCraft DSP").
//! Process PID: `~/.config/gearcraft/dsp/wpexec.pid`

use std::fs;
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;

use gearcraft_core::AudioEffects;
use tokio::process::Command;
use tracing::{info, warn};

const NODE_NAME: &str = "gearcraft.dsp";
const SINK_DESC: &str = "GearCraft DSP";

#[derive(Debug, Clone)]
pub struct DspApplyResult {
    pub applied: bool,
    pub eq: bool,
    pub crossfeed: bool,
    pub conf_path: String,
    pub message: String,
}

fn dsp_dir() -> PathBuf {
    let d = crate::config::config_dir().join("dsp");
    let _ = fs::create_dir_all(&d);
    d
}

fn lua_script_path() -> PathBuf {
    dsp_dir().join("gearcraft-dsp.lua")
}

fn args_json_path() -> PathBuf {
    dsp_dir().join("filter-chain.json")
}

fn pid_path() -> PathBuf {
    dsp_dir().join("wpexec.pid")
}

fn band_q() -> f32 {
    1.41
}

/// Build SPA-JSON args for libpipewire-module-filter-chain (WirePlumber LocalModule).
pub fn build_filter_chain_json(fx: &AudioEffects) -> String {
    let mut nodes: Vec<String> = Vec::new();
    let mut links: Vec<String> = Vec::new();
    let mut prev: Option<String> = None;

    let mut preamp = 0.0_f32;
    if fx.eq_enabled {
        let max_boost = fx
            .eq_bands
            .iter()
            .map(|b| b.gain_db)
            .fold(0.0_f32, f32::max);
        if max_boost > 0.0 {
            preamp = -max_boost.min(12.0) * 0.5;
        }
    }
    if fx.bass_boost {
        preamp -= fx.bass_boost_db.max(0.0) * 0.25;
    }
    if fx.limiter {
        preamp = preamp.min(fx.limiter_db);
    }

    let mut push = |name: &str, label: &str, freq: f32, q: f32, gain: f32| {
        nodes.push(format!(
            r#"{{"type":"builtin","name":"{name}","label":"{label}","control":{{"Freq":{freq:.2},"Q":{q:.3},"Gain":{gain:.2}}}}"#
        ));
        if let Some(ref p) = prev {
            links.push(format!(
                r#"{{"output":"{p}:Out","input":"{name}:In"}}"#
            ));
        }
        prev = Some(name.to_string());
    };

    if preamp.abs() > 0.01 {
        push("gc_preamp", "bq_highshelf", 0.0, 1.0, preamp);
    }

    if fx.eq_enabled {
        for (i, band) in fx.eq_bands.iter().enumerate() {
            if band.gain_db.abs() < 0.05 {
                continue;
            }
            let name = format!("gc_eq_{i}");
            let label = if i == 0 && band.freq_hz <= 40.0 {
                "bq_lowshelf"
            } else {
                "bq_peaking"
            };
            push(&name, label, band.freq_hz, band_q(), band.gain_db);
        }
    }

    if fx.bass_boost && fx.bass_boost_db.abs() > 0.05 {
        push("gc_bass", "bq_lowshelf", 80.0, 0.7, fx.bass_boost_db);
    }

    if fx.crossfeed && fx.crossfeed_strength > 0 {
        if let Some(plugin) = find_bs2b_plugin() {
            let level = 1.0 + (fx.crossfeed_strength as f32 / 100.0) * 5.0;
            let name = "gc_bs2b";
            nodes.push(format!(
                r#"{{"type":"ladspa","name":"{name}","plugin":"{plugin}","label":"bs2b","control":{{"Level":{level:.2}}}}"#
            ));
            if let Some(ref p) = prev {
                links.push(format!(
                    r#"{{"output":"{p}:Out","input":"{name}:In"}}"#
                ));
            }
            prev = Some(name.to_string());
            let _ = prev;
        } else {
            let mix = (fx.crossfeed_strength as f32 / 100.0) * 0.35;
            let g1 = 1.0 - mix;
            let g2 = mix;
            let name = "gc_xf_mix";
            nodes.push(format!(
                r#"{{"type":"builtin","name":"{name}","label":"mixer","control":{{"Gain 1":{g1:.3},"Gain 2":{g2:.3}}}}"#
            ));
            if let Some(ref p) = prev {
                links.push(format!(
                    r#"{{"output":"{p}:Out","input":"{name}:In 1"}}"#
                ));
                links.push(format!(
                    r#"{{"output":"{p}:Out","input":"{name}:In 2"}}"#
                ));
            }
        }
    }

    if nodes.is_empty() {
        nodes.push(
            r#"{"type":"builtin","name":"gc_thru","label":"copy"}"#.into(),
        );
    }

    format!(
        r#"{{
  "node.description": "{SINK_DESC}",
  "media.name": "{SINK_DESC}",
  "audio.channels": 2,
  "audio.position": ["FL", "FR"],
  "filter.graph": {{
    "nodes": [
      {nodes}
    ],
    "links": [
      {links}
    ]
  }},
  "capture.props": {{
    "node.name": "{NODE_NAME}",
    "media.class": "Audio/Sink",
    "filter.smart": true,
    "filter.smart.name": "gearcraft-dsp"
  }},
  "playback.props": {{
    "node.name": "{NODE_NAME}.output",
    "node.passive": true
  }}
}}"#,
        nodes = nodes.join(",\n      "),
        links = links.join(",\n      "),
    )
}

fn find_bs2b_plugin() -> Option<String> {
    for c in [
        "/usr/lib/ladspa/bs2b.so",
        "/usr/lib64/ladspa/bs2b.so",
        "/usr/local/lib/ladspa/bs2b.so",
        "/usr/lib/x86_64-linux-gnu/ladspa/bs2b.so",
    ] {
        if PathBuf::from(c).exists() {
            return Some(c.to_string());
        }
    }
    None
}

fn build_wpexec_lua(args_json: &str) -> String {
    format!(
        r#"#!/usr/bin/env wpexec
-- GearCraft DSP — WirePlumber filter-chain loader (generated, do not edit)
-- Sink: {NODE_NAME} ("{SINK_DESC}")

local args_string = [=======[
{args_json}
]=======]

print("GearCraft: loading libpipewire-module-filter-chain via WirePlumber")
filter_chain = LocalModule("libpipewire-module-filter-chain", args_string, {{}})
print("GearCraft: filter-chain active (" .. "{NODE_NAME}" .. ")")

Core.idle_add(function()
  return true
end)
"#
    )
}

async fn stop_running_wpexec() {
    let path = pid_path();
    if let Ok(s) = fs::read_to_string(&path) {
        if let Ok(pid) = s.trim().parse::<i32>() {
            if pid > 1 {
                let _ = Command::new("kill")
                    .args(["-TERM", &pid.to_string()])
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status()
                    .await;
                tokio::time::sleep(Duration::from_millis(200)).await;
                let _ = Command::new("kill")
                    .args(["-KILL", &pid.to_string()])
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status()
                    .await;
                info!("stopped previous wpexec pid={pid}");
            }
        }
    }
    let _ = fs::remove_file(&path);
    let _ = Command::new("pkill")
        .args(["-f", "gearcraft-dsp.lua"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await;
}

pub async fn apply_effects(fx: &AudioEffects) -> Result<DspApplyResult, String> {
    if fx.bypass {
        return clear_effects().await;
    }

    let eq_on = fx.eq_enabled && fx.eq_bands.iter().any(|b| b.gain_db.abs() >= 0.05);
    let xf_on = fx.crossfeed && fx.crossfeed_strength > 0;
    let bass_on = fx.bass_boost && fx.bass_boost_db.abs() >= 0.05;

    if !eq_on && !xf_on && !bass_on {
        return clear_effects().await;
    }

    if !wpexec_available().await {
        return Err(
            "wpexec not found — install wireplumber (provides /usr/bin/wpexec)".into(),
        );
    }

    let json = build_filter_chain_json(fx);
    let json_path = args_json_path();
    fs::write(&json_path, &json).map_err(|e| format!("write {}: {e}", json_path.display()))?;

    let lua = build_wpexec_lua(&json);
    let lua_path = lua_script_path();
    fs::write(&lua_path, &lua).map_err(|e| format!("write {}: {e}", lua_path.display()))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&lua_path, fs::Permissions::from_mode(0o755));
    }

    stop_running_wpexec().await;

    let child = Command::new("wpexec")
        .arg(&lua_path)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(false)
        .spawn()
        .map_err(|e| format!("spawn wpexec: {e}"))?;

    let pid = child.id().ok_or_else(|| "wpexec pid unavailable".to_string())?;
    fs::write(pid_path(), pid.to_string()).map_err(|e| format!("write pid: {e}"))?;
    std::mem::forget(child);

    tokio::time::sleep(Duration::from_millis(350)).await;

    let mut message = format!(
        "WirePlumber filter-chain loaded (wpexec pid {pid}); sink «{SINK_DESC}» ({NODE_NAME})"
    );
    if xf_on && find_bs2b_plugin().is_none() {
        message.push_str("; crossfeed: soft mixer (install ladspa-bs2b for BS2B)");
    } else if xf_on {
        message.push_str("; crossfeed: LADSPA bs2b");
    }

    if std::env::var("GEARCRAFT_DSP_DEFAULT").as_deref() == Ok("1") {
        if let Some(id) = find_sink_id_wpctl().await {
            let _ = Command::new("wpctl")
                .args(["set-default", &id])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .await;
            message.push_str(&format!("; wpctl set-default {id}"));
        }
    }

    info!("WirePlumber DSP applied eq={eq_on} bass={bass_on} xf={xf_on} pid={pid}");

    Ok(DspApplyResult {
        applied: true,
        eq: eq_on || bass_on,
        crossfeed: xf_on,
        conf_path: lua_path.display().to_string(),
        message,
    })
}

pub async fn clear_effects() -> Result<DspApplyResult, String> {
    stop_running_wpexec().await;
    let _ = fs::remove_file(lua_script_path());
    Ok(DspApplyResult {
        applied: false,
        eq: false,
        crossfeed: false,
        conf_path: lua_script_path().display().to_string(),
        message: "WirePlumber DSP cleared (wpexec stopped)".into(),
    })
}

async fn wpexec_available() -> bool {
    Command::new("wpexec")
        .arg("--help")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await
        .map(|s| s.success() || s.code() == Some(1) || s.code() == Some(2))
        .unwrap_or(false)
        || which_exists("wpexec")
}

fn which_exists(bin: &str) -> bool {
    std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).any(|dir| dir.join(bin).exists()))
        .unwrap_or(false)
}

async fn find_sink_id_wpctl() -> Option<String> {
    let out = Command::new("wpctl")
        .arg("status")
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .await
        .ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    for line in text.lines() {
        if line.contains(SINK_DESC) || line.contains(NODE_NAME) {
            let trimmed = line.trim_start_matches(['│', ' ', '*', '.']);
            let id: String = trimmed
                .chars()
                .take_while(|c| c.is_ascii_digit())
                .collect();
            if !id.is_empty() {
                return Some(id);
            }
            for tok in line.split_whitespace() {
                let t = tok.trim_matches(|c: char| !c.is_ascii_digit());
                if !t.is_empty() && t.chars().all(|c| c.is_ascii_digit()) {
                    return Some(t.to_string());
                }
            }
        }
    }
    None
}

pub async fn dsp_status() -> serde_json::Value {
    let pid_alive = fs::read_to_string(pid_path())
        .ok()
        .and_then(|s| s.trim().parse::<i32>().ok())
        .map(|pid| PathBuf::from(format!("/proc/{pid}")).exists())
        .unwrap_or(false);

    serde_json::json!({
        "backend": "wireplumber-wpexec",
        "wpexec": which_exists("wpexec"),
        "wpctl": which_exists("wpctl"),
        "running": pid_alive,
        "lua_script": lua_script_path().display().to_string(),
        "args_json": args_json_path().display().to_string(),
        "node_name": NODE_NAME,
        "sink_description": SINK_DESC,
        "bs2b_ladspa": find_bs2b_plugin().is_some(),
        "filter_smart": true,
    })
}
