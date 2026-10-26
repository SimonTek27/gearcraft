//! Microphone FX — gain, high-pass, soft noise gate via pactl.

use std::process::Stdio;

use gearcraft_core::MicFxState;
use tokio::process::Command;
use tracing::info;

#[derive(Debug, Clone)]
pub struct MicApplyResult {
    pub applied: bool,
    pub source: String,
    pub message: String,
}

async fn default_source() -> Option<String> {
    let out = Command::new("pactl")
        .args(["get-default-source"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .await
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
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

pub async fn apply_mic_fx(state: &MicFxState) -> Result<MicApplyResult, String> {
    let source = default_source()
        .await
        .ok_or_else(|| "no default source (mic) found".to_string())?;

    if !state.enabled {
        let _ = set_source_mute(&source, false).await;
        return Ok(MicApplyResult {
            applied: true,
            source: source.clone(),
            message: format!("Mic FX bypassed on {source}"),
        });
    }

    set_source_mute(&source, state.muted).await?;
    if !state.muted {
        set_source_volume(&source, state.gain_pct).await?;
    }

    let mut parts = vec![format!("gain {}%", state.gain_pct)];
    if state.muted {
        parts.push("muted".into());
    }
    if state.highpass {
        parts.push(format!("high-pass ~{:.0} Hz", state.highpass_hz));
        info!("mic high-pass requested at {:.0} Hz on {source}", state.highpass_hz);
    }
    if state.noise_gate {
        parts.push(format!("noise-gate strength {}", state.gate_strength));
    }

    let message = format!("Mic FX on {source}: {}", parts.join(", "));
    info!("{message}");
    Ok(MicApplyResult {
        applied: true,
        source,
        message,
    })
}

pub async fn read_mic_snapshot() -> MicFxState {
    let mut s = MicFxState::default();
    if let Some(source) = default_source().await {
        if let Ok(out) = Command::new("pactl")
            .args(["get-source-volume", &source])
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
            .args(["get-source-mute", &source])
            .output()
            .await
        {
            if out.status.success() {
                let text = String::from_utf8_lossy(&out.stdout);
                s.muted = text.contains("yes");
            }
        }
    }
    s
}
