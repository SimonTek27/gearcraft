//! Software ChatMix via PipeWire (wpctl) or PulseAudio compatibility (pactl).
//!
//! Expected sink names (override with env):
//! - `GEARCRAFT_GAME_SINK` (default: look for "Game" / "gearcraft_game")
//! - `GEARCRAFT_CHAT_SINK` (default: look for "Chat" / "gearcraft_chat")
//!
//! Create virtual sinks:
//! ```bash
//! pactl load-module module-null-sink sink_name=gearcraft_game \
//!   sink_properties=device.description=GearCraft_Game
//! pactl load-module module-null-sink sink_name=gearcraft_chat \
//!   sink_properties=device.description=GearCraft_Chat
//! ```
//! Then route game apps → GearCraft_Game and chat apps → GearCraft_Chat
//! (EasyEffects / Helvum / pavucontrol).

use std::process::Stdio;

use tokio::process::Command;
use tracing::{debug, info, warn};

#[derive(Debug, Clone)]
pub struct AudioBackendInfo {
    pub backend: String,
    pub game_sink: Option<String>,
    pub chat_sink: Option<String>,
    pub available: bool,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tool {
    Wpctl,
    Pactl,
}

async fn which_tool() -> Option<Tool> {
    if run_ok("wpctl", &["--version"]).await {
        return Some(Tool::Wpctl);
    }
    if run_ok("pactl", &["--version"]).await {
        return Some(Tool::Pactl);
    }
    None
}

async fn run_ok(bin: &str, args: &[&str]) -> bool {
    Command::new(bin)
        .args(args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await
        .map(|s| s.success())
        .unwrap_or(false)
}

async fn run_out(bin: &str, args: &[&str]) -> Result<String, String> {
    let out = Command::new(bin)
        .args(args)
        .output()
        .await
        .map_err(|e| format!("{bin}: {e}"))?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        return Err(format!("{bin} failed: {err}"));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.into())
}

/// Resolve game/chat sink identifiers for the active backend.
pub async fn detect_sinks() -> AudioBackendInfo {
    let tool = which_tool().await;
    match tool {
        None => AudioBackendInfo {
            backend: "none".into(),
            game_sink: None,
            chat_sink: None,
            available: false,
            message: "Neither wpctl nor pactl found – install pipewire-pulse or wireplumber"
                .into(),
        },
        Some(Tool::Wpctl) => detect_wpctl().await,
        Some(Tool::Pactl) => detect_pactl().await,
    }
}

async fn detect_wpctl() -> AudioBackendInfo {
    let status = run_out("wpctl", &["status"]).await.unwrap_or_default();
    let game_pref = env_or("GEARCRAFT_GAME_SINK", "");
    let chat_pref = env_or("GEARCRAFT_CHAT_SINK", "");

    let game = find_wpctl_id(&status, &game_pref, &["gearcraft_game", "Game", "GAME"]);
    let chat = find_wpctl_id(&status, &chat_pref, &["gearcraft_chat", "Chat", "CHAT"]);

    let available = game.is_some() && chat.is_some();
    let message = if available {
        format!(
            "wpctl: game={} chat={}",
            game.as_deref().unwrap_or("?"),
            chat.as_deref().unwrap_or("?")
        )
    } else {
        "wpctl found but Game/Chat sinks missing – run setup or set GEARCRAFT_*_SINK".into()
    };

    AudioBackendInfo {
        backend: "wpctl".into(),
        game_sink: game,
        chat_sink: chat,
        available,
        message,
    }
}

/// Parse `wpctl status` lines like `    56. GearCraft_Game ...`
fn find_wpctl_id(status: &str, preferred: &str, names: &[&str]) -> Option<String> {
    if !preferred.is_empty() {
        if preferred.chars().all(|c| c.is_ascii_digit()) {
            return Some(preferred.to_string());
        }
        for line in status.lines() {
            if line.to_ascii_lowercase().contains(&preferred.to_ascii_lowercase()) {
                if let Some(id) = extract_wpctl_id(line) {
                    return Some(id);
                }
            }
        }
    }
    for name in names {
        for line in status.lines() {
            if line.to_ascii_lowercase().contains(&name.to_ascii_lowercase()) {
                if let Some(id) = extract_wpctl_id(line) {
                    return Some(id);
                }
            }
        }
    }
    None
}

fn extract_wpctl_id(line: &str) -> Option<String> {
    let t = line.trim().trim_start_matches('│').trim();
    let num: String = t.chars().take_while(|c| c.is_ascii_digit()).collect();
    if num.is_empty() {
        None
    } else {
        Some(num)
    }
}

async fn detect_pactl() -> AudioBackendInfo {
    let list = run_out("pactl", &["list", "short", "sinks"])
        .await
        .unwrap_or_default();
    let game_pref = env_or("GEARCRAFT_GAME_SINK", "gearcraft_game");
    let chat_pref = env_or("GEARCRAFT_CHAT_SINK", "gearcraft_chat");

    let game = find_pactl_sink(&list, &game_pref, &["gearcraft_game", "Game"]);
    let chat = find_pactl_sink(&list, &chat_pref, &["gearcraft_chat", "Chat"]);

    let available = game.is_some() && chat.is_some();
    AudioBackendInfo {
        backend: "pactl".into(),
        game_sink: game,
        chat_sink: chat,
        available,
        message: if available {
            "pactl sinks ready".into()
        } else {
            "pactl found but gearcraft_game/chat sinks missing – call setup_chatmix_sinks".into()
        },
    }
}

fn find_pactl_sink(list: &str, preferred: &str, alts: &[&str]) -> Option<String> {
    let mut candidates = vec![preferred];
    candidates.extend(alts);
    for name in candidates {
        for line in list.lines() {
            let cols: Vec<_> = line.split_whitespace().collect();
            if cols.len() >= 2 {
                let sink_name = cols[1];
                if sink_name.eq_ignore_ascii_case(name)
                    || sink_name.to_ascii_lowercase().contains(&name.to_ascii_lowercase())
                {
                    return Some(sink_name.to_string());
                }
            }
        }
    }
    None
}

/// Apply linear volumes 0–100 to game and chat sinks.
pub async fn apply_volumes(game_pct: u8, chat_pct: u8) -> Result<AudioBackendInfo, String> {
    let info = detect_sinks().await;
    if !info.available {
        return Err(info.message);
    }

    match info.backend.as_str() {
        "wpctl" => {
            let g = info.game_sink.as_deref().unwrap();
            let c = info.chat_sink.as_deref().unwrap();
            let gv = format!("{:.2}", game_pct as f32 / 100.0);
            let cv = format!("{:.2}", chat_pct as f32 / 100.0);
            run_out("wpctl", &["set-volume", g, &gv]).await?;
            run_out("wpctl", &["set-volume", c, &cv]).await?;
            info!("wpctl volumes game={game_pct}% chat={chat_pct}%");
        }
        "pactl" => {
            let g = info.game_sink.as_deref().unwrap();
            let c = info.chat_sink.as_deref().unwrap();
            let gv = format!("{}%", game_pct);
            let cv = format!("{}%", chat_pct);
            run_out("pactl", &["set-sink-volume", g, &gv]).await?;
            run_out("pactl", &["set-sink-volume", c, &cv]).await?;
            info!("pactl volumes game={game_pct}% chat={chat_pct}%");
        }
        _ => return Err("no audio backend".into()),
    }
    Ok(info)
}

/// Create null sinks for Game and Chat if missing (pactl only).
pub async fn setup_virtual_sinks() -> Result<String, String> {
    if !run_ok("pactl", &["--version"]).await {
        return Err(
            "pactl required to create null sinks (install pipewire-pulse)".into(),
        );
    }

    let list = run_out("pactl", &["list", "short", "sinks"])
        .await
        .unwrap_or_default();

    let mut created = Vec::new();
    for (name, desc) in [
        ("gearcraft_game", "GearCraft_Game"),
        ("gearcraft_chat", "GearCraft_Chat"),
    ] {
        if list.lines().any(|l| l.contains(name)) {
            debug!("sink {name} already exists");
            continue;
        }
        let prop = format!("device.description={desc}");
        run_out(
            "pactl",
            &[
                "load-module",
                "module-null-sink",
                &format!("sink_name={name}"),
                &format!("sink_properties={prop}"),
            ],
        )
        .await?;
        created.push(name);
        info!("created null sink {name}");
    }

    if std::env::var("GEARCRAFT_CHATMIX_LOOPBACK").as_deref() == Ok("1") {
        for name in ["gearcraft_game", "gearcraft_chat"] {
            let _ = run_out(
                "pactl",
                &[
                    "load-module",
                    "module-loopback",
                    &format!("source={name}.monitor"),
                    "latency_msec=20",
                ],
            )
            .await;
        }
    }

    if created.is_empty() {
        Ok("sinks already present".into())
    } else {
        Ok(format!("created: {}", created.join(", ")))
    }
}
