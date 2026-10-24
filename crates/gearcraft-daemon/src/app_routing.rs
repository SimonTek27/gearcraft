//! Automatic application → Sonar channel routing (PipeWire / pactl).
//!
//! Classifies running sink-inputs by `application.process.binary` /
//! `application.name` and moves them onto the matching GearCraft virtual sink:
//! - CHAT  → gearcraft_chat   (Discord, Teams, …)
//! - MEDIA → gearcraft_media  (Haruna, VLC, Spotify, …)
//! - GAME  → gearcraft_game   (Steam, Wine, …)
//! - AUX   → gearcraft_aux    (OBS, …)
//!
//! Rules: built-in defaults + optional `~/.config/gearcraft/app_routes.json`.

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;

use serde::{Deserialize, Serialize};
use tokio::process::Command;
use tracing::{debug, info, warn};

/// Sonar channel target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Channel {
    Game,
    Chat,
    Media,
    Aux,
    /// Leave on current / system default
    Ignore,
}

impl Channel {
    pub fn sink_name(self) -> Option<&'static str> {
        match self {
            Channel::Game => Some("gearcraft_game"),
            Channel::Chat => Some("gearcraft_chat"),
            Channel::Media => Some("gearcraft_media"),
            Channel::Aux => Some("gearcraft_aux"),
            Channel::Ignore => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Channel::Game => "game",
            Channel::Chat => "chat",
            Channel::Media => "media",
            Channel::Aux => "aux",
            Channel::Ignore => "ignore",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteRule {
    /// Substring match (case-insensitive) against binary or app name
    pub match_pattern: String,
    pub channel: Channel,
    #[serde(default)]
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppRouteConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub rules: Vec<RouteRule>,
}

fn default_true() -> bool {
    true
}

impl Default for AppRouteConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            rules: builtin_rules(),
        }
    }
}

fn routes_path() -> PathBuf {
    crate::config::config_dir().join("app_routes.json")
}

pub fn load_config() -> AppRouteConfig {
    let path = routes_path();
    match std::fs::read_to_string(&path) {
        Ok(data) => match serde_json::from_str::<AppRouteConfig>(&data) {
            Ok(mut cfg) => {
                if cfg.rules.is_empty() {
                    cfg.rules = builtin_rules();
                }
                info!(
                    "app routing: {} rule(s) from {}",
                    cfg.rules.len(),
                    path.display()
                );
                cfg
            }
            Err(e) => {
                warn!("app_routes.json parse error: {e}; using builtins");
                AppRouteConfig::default()
            }
        },
        Err(_) => {
            let cfg = AppRouteConfig::default();
            let _ = save_config(&cfg);
            cfg
        }
    }
}

pub fn save_config(cfg: &AppRouteConfig) -> Result<(), String> {
    let path = routes_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let data = serde_json::to_string_pretty(cfg).map_err(|e| e.to_string())?;
    std::fs::write(&path, data).map_err(|e| e.to_string())
}

fn builtin_rules() -> Vec<RouteRule> {
    let mut r = Vec::new();
    let mut add = |patterns: &[&str], ch: Channel, note: &str| {
        for p in patterns {
            r.push(RouteRule {
                match_pattern: (*p).into(),
                channel: ch,
                note: note.into(),
            });
        }
    };

    add(
        &[
            "discord", "webcord", "vesktop", "teamspeak", "ts3client", "mumble",
            "zoom", "slack", "telegram", "element", "signal", "skype", "jitsi",
            "hexchat", "teams", "webex",
        ],
        Channel::Chat,
        "voice / chat",
    );

    add(
        &[
            "haruna", "vlc", "mpv", "celluloid", "smplayer", "totem", "dragon",
            "spotify", "cider", "rhythmbox", "strawberry", "audacious", "elisa",
            "amarok", "lollypop", "tauon", "deadbeef", "jellyfinmediaplayer",
            "plexmediaplayer",
        ],
        Channel::Media,
        "media player",
    );

    add(
        &["obs", "obs-studio", "obs64", "streamlabs"],
        Channel::Aux,
        "broadcast / aux",
    );

    add(
        &[
            "steam", "steamwebhelper", "wine", "lutris", "heroic", "legendary",
            "gamescope", "minecraft", "javaw", "cs2", "dota", "riotclient",
            "league", "battle.net", "origin", "uplay", "epicgameslauncher",
        ],
        Channel::Game,
        "game / launcher",
    );

    r
}

fn classify(name: &str, binary: &str, rules: &[RouteRule]) -> Channel {
    let hay = format!(
        "{} {}",
        name.to_ascii_lowercase(),
        binary.to_ascii_lowercase()
    );
    for rule in rules {
        let pat = rule.match_pattern.to_ascii_lowercase();
        if !pat.is_empty() && hay.contains(&pat) {
            return rule.channel;
        }
    }
    Channel::Ignore
}

#[derive(Debug, Clone)]
struct SinkInput {
    id: u32,
    app_name: String,
    binary: String,
    sink: String,
}

async fn list_sink_inputs() -> Result<Vec<SinkInput>, String> {
    let out = Command::new("pactl")
        .args(["list", "sink-inputs"])
        .output()
        .await
        .map_err(|e| format!("pactl: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "pactl list sink-inputs failed: {}",
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let mut items = Vec::new();
    let mut cur_id: Option<u32> = None;
    let mut app_name = String::new();
    let mut binary = String::new();
    let mut sink = String::new();

    let flush = |items: &mut Vec<SinkInput>,
                 cur_id: &mut Option<u32>,
                 app_name: &mut String,
                 binary: &mut String,
                 sink: &mut String| {
        if let Some(id) = cur_id.take() {
            items.push(SinkInput {
                id,
                app_name: std::mem::take(app_name),
                binary: std::mem::take(binary),
                sink: std::mem::take(sink),
            });
        }
    };

    for line in text.lines() {
        let t = line.trim();
        if let Some(rest) = t.strip_prefix("Sink Input #") {
            flush(
                &mut items,
                &mut cur_id,
                &mut app_name,
                &mut binary,
                &mut sink,
            );
            cur_id = rest.trim().parse().ok();
        } else if let Some(v) = t.strip_prefix("Sink:") {
            sink = v.trim().to_string();
        } else if let Some(v) = t
            .strip_prefix("application.name = ")
            .or_else(|| t.strip_prefix("application.name ="))
        {
            app_name = v.trim().trim_matches('"').to_string();
        } else if let Some(v) = t
            .strip_prefix("application.process.binary = ")
            .or_else(|| t.strip_prefix("application.process.binary ="))
        {
            binary = v.trim().trim_matches('"').to_string();
        }
    }
    flush(
        &mut items,
        &mut cur_id,
        &mut app_name,
        &mut binary,
        &mut sink,
    );
    Ok(items)
}

async fn sink_exists(name: &str) -> bool {
    let out = Command::new("pactl")
        .args(["list", "short", "sinks"])
        .output()
        .await
        .ok();
    out.map(|o| String::from_utf8_lossy(&o.stdout).contains(name))
        .unwrap_or(false)
}

async fn move_input(id: u32, sink: &str) -> Result<(), String> {
    let status = Command::new("pactl")
        .args(["move-sink-input", &id.to_string(), sink])
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|e| e.to_string())?;
    if !status.status.success() {
        return Err(String::from_utf8_lossy(&status.stderr).into_owned());
    }
    Ok(())
}

#[derive(Debug, Default, Serialize)]
pub struct RoutePassReport {
    pub examined: usize,
    pub moved: usize,
    pub skipped: usize,
    pub actions: Vec<String>,
}

pub async fn route_pass(cfg: &AppRouteConfig) -> RoutePassReport {
    let mut report = RoutePassReport::default();
    if !cfg.enabled {
        return report;
    }

    let inputs = match list_sink_inputs().await {
        Ok(v) => v,
        Err(e) => {
            debug!("app routing scan skipped: {e}");
            return report;
        }
    };

    report.examined = inputs.len();
    let mut sink_ok: HashMap<String, bool> = HashMap::new();

    for inp in inputs {
        let ch = classify(&inp.app_name, &inp.binary, &cfg.rules);
        let Some(target) = ch.sink_name() else {
            report.skipped += 1;
            continue;
        };

        if inp.sink.contains(target) {
            report.skipped += 1;
            continue;
        }

        let ok = if let Some(v) = sink_ok.get(target) {
            *v
        } else {
            let v = sink_exists(target).await;
            sink_ok.insert(target.to_string(), v);
            v
        };
        if !ok {
            report.skipped += 1;
            continue;
        }

        match move_input(inp.id, target).await {
            Ok(()) => {
                let label = if inp.app_name.is_empty() {
                    inp.binary.as_str()
                } else {
                    inp.app_name.as_str()
                };
                let msg = format!(
                    "#{} «{}» ({}) → {} [{}]",
                    inp.id,
                    label,
                    inp.binary,
                    target,
                    ch.as_str()
                );
                info!("app route: {msg}");
                report.actions.push(msg);
                report.moved += 1;
            }
            Err(e) => {
                debug!("move-sink-input #{} failed: {e}", inp.id);
                report.skipped += 1;
            }
        }
    }
    report
}

pub async fn run_loop(interval_secs: u64) {
    let mut cfg = load_config();
    let mut ticks: u64 = 0;
    let mut interval =
        tokio::time::interval(std::time::Duration::from_secs(interval_secs.max(1)));
    loop {
        interval.tick().await;
        ticks += 1;
        if ticks % 30 == 0 {
            cfg = load_config();
        }
        let report = route_pass(&cfg).await;
        if report.moved > 0 {
            info!(
                "app routing: moved={} examined={}",
                report.moved, report.examined
            );
        }
    }
}
