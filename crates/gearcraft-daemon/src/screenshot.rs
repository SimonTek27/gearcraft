//! Screen capture — Spectacle-style (fullscreen / window / region).
//!
//! Backend preference:
//! 1. `spectacle` (KDE) — native flags
//! 2. `gnome-screenshot`
//! 3. Wayland: `grim` (+ `slurp` for region)
//! 4. X11: `maim` / `scrot` / ImageMagick `import`

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::{SystemTime, UNIX_EPOCH};

use tokio::process::Command;
use tracing::{info, warn};

#[derive(Debug, Clone)]
pub struct CaptureResult {
    pub path: Option<String>,
    pub backend: String,
    pub message: String,
}

fn pictures_dir() -> PathBuf {
    if let Ok(p) = std::env::var("XDG_PICTURES_DIR") {
        return PathBuf::from(p).join("GearCraft");
    }
    if let Ok(home) = std::env::var("HOME") {
        let p = PathBuf::from(home).join("Pictures").join("GearCraft");
        return p;
    }
    PathBuf::from("/tmp/gearcraft-screenshots")
}

fn timestamp_name() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("gearcraft-{secs}.png")
}

async fn which(bin: &str) -> bool {
    Command::new("which")
        .arg(bin)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await
        .map(|s| s.success())
        .unwrap_or(false)
}

async fn run(cmd: &str, args: &[&str]) -> Result<(), String> {
    let out = Command::new(cmd)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|e| format!("{cmd}: {e}"))?;
    if out.status.success() {
        Ok(())
    } else {
        let err = String::from_utf8_lossy(&out.stderr);
        Err(format!("{cmd} failed: {err}"))
    }
}

async fn copy_to_clipboard(path: &Path) -> Result<(), String> {
    if which("wl-copy").await {
        let out = Command::new("wl-copy")
            .arg("--type")
            .arg("image/png")
            .arg("--")
            .stdin(Stdio::piped())
            .spawn();
        // simpler: wl-copy < file
        let status = Command::new("sh")
            .arg("-c")
            .arg(format!("wl-copy --type image/png < '{}'", path.display()))
            .status()
            .await
            .map_err(|e| e.to_string())?;
        if status.success() {
            return Ok(());
        }
        let _ = out;
    }
    if which("xclip").await {
        let status = Command::new("xclip")
            .args(["-selection", "clipboard", "-t", "image/png", "-i"])
            .arg(path.as_os_str())
            .status()
            .await
            .map_err(|e| e.to_string())?;
        if status.success() {
            return Ok(());
        }
    }
    Err("no wl-copy/xclip for clipboard".into())
}

/// Capture using available tools. `mode`: full | window | region
pub async fn capture(
    mode: &str,
    delay_secs: u8,
    copy_clipboard: bool,
    save_dir: Option<&str>,
) -> Result<CaptureResult, String> {
    let dir = save_dir
        .map(PathBuf::from)
        .unwrap_or_else(pictures_dir);
    std::fs::create_dir_all(&dir).map_err(|e| format!("mkdir: {e}"))?;
    let path = dir.join(timestamp_name());
    let path_s = path.to_string_lossy().to_string();

    if delay_secs > 0 {
        info!("screenshot delay {delay_secs}s");
        tokio::time::sleep(std::time::Duration::from_secs(delay_secs as u64)).await;
    }

    let mode = mode.to_ascii_lowercase();
    let mut backend = String::new();
    let mut last_err = String::new();

    // 1) KDE Spectacle
    if which("spectacle").await {
        let mut args: Vec<&str> = vec!["-b", "-n"]; // background, no GUI notify optional
        // -b background; -c clipboard later; -o output
        match mode.as_str() {
            "window" => args.extend(["-a"]),           // active window
            "region" => args.extend(["-r"]),           // region
            _ => args.extend(["-f"]),                  // fullscreen
        }
        args.extend(["-o", path_s.as_str()]);
        match run("spectacle", &args).await {
            Ok(()) if path.exists() => {
                backend = "spectacle".into();
            }
            Ok(()) => last_err = "spectacle: file not written".into(),
            Err(e) => last_err = e,
        }
    }

    // 2) gnome-screenshot
    if backend.is_empty() && which("gnome-screenshot").await {
        let mut args = vec!["-f", path_s.as_str()];
        match mode.as_str() {
            "window" => args.insert(0, "-w"),
            "region" => args.insert(0, "-a"),
            _ => {}
        }
        match run("gnome-screenshot", &args).await {
            Ok(()) if path.exists() => backend = "gnome-screenshot".into(),
            Ok(()) => last_err = "gnome-screenshot: file not written".into(),
            Err(e) => last_err = e,
        }
    }

    // 3) grim (+ slurp)
    if backend.is_empty() && which("grim").await {
        match mode.as_str() {
            "region" if which("slurp").await => {
                let geom = Command::new("slurp")
                    .stdout(Stdio::piped())
                    .output()
                    .await
                    .map_err(|e| format!("slurp: {e}"))?;
                if geom.status.success() {
                    let g = String::from_utf8_lossy(&geom.stdout).trim().to_string();
                    if !g.is_empty() {
                        match run("grim", &["-g", &g, path_s.as_str()]).await {
                            Ok(()) if path.exists() => backend = "grim+slurp".into(),
                            Ok(()) => last_err = "grim: file not written".into(),
                            Err(e) => last_err = e,
                        }
                    }
                } else {
                    last_err = "slurp cancelled or failed".into();
                }
            }
            "window" => {
                // grim doesn't do active window alone; fallback fullscreen
                match run("grim", &[path_s.as_str()]).await {
                    Ok(()) if path.exists() => {
                        backend = "grim".into();
                        warn!("grim: window mode not supported, captured full screen");
                    }
                    Ok(()) => last_err = "grim: file not written".into(),
                    Err(e) => last_err = e,
                }
            }
            _ => match run("grim", &[path_s.as_str()]).await {
                Ok(()) if path.exists() => backend = "grim".into(),
                Ok(()) => last_err = "grim: file not written".into(),
                Err(e) => last_err = e,
            },
        }
    }

    // 4) maim
    if backend.is_empty() && which("maim").await {
        let args: Vec<&str> = match mode.as_str() {
            "window" => vec!["-i", "$(xdotool getactivewindow)", path_s.as_str()],
            "region" => vec!["-s", path_s.as_str()],
            _ => vec![path_s.as_str()],
        };
        if mode == "window" && which("xdotool").await {
            let status = Command::new("sh")
                .arg("-c")
                .arg(format!(
                    "maim -i \"$(xdotool getactivewindow)\" '{}'",
                    path.display()
                ))
                .status()
                .await
                .map_err(|e| e.to_string())?;
            if status.success() && path.exists() {
                backend = "maim".into();
            }
        } else {
            match run("maim", &args).await {
                Ok(()) if path.exists() => backend = "maim".into(),
                Ok(()) => last_err = "maim: file not written".into(),
                Err(e) => last_err = e,
            }
        }
    }

    // 5) scrot
    if backend.is_empty() && which("scrot").await {
        let args: Vec<&str> = match mode.as_str() {
            "window" => vec!["-u", path_s.as_str()],
            "region" => vec!["-s", path_s.as_str()],
            _ => vec![path_s.as_str()],
        };
        match run("scrot", &args).await {
            Ok(()) if path.exists() => backend = "scrot".into(),
            Ok(()) => last_err = "scrot: file not written".into(),
            Err(e) => last_err = e,
        }
    }

    if backend.is_empty() {
        return Err(format!(
            "no screenshot backend (install spectacle, grim, maim, or scrot). last: {last_err}"
        ));
    }

    let mut message = format!("Saved {path_s} via {backend}");
    if copy_clipboard {
        match copy_to_clipboard(&path).await {
            Ok(()) => message.push_str(" · copied to clipboard"),
            Err(e) => {
                warn!("{e}");
                message.push_str(&format!(" · clipboard skipped ({e})"));
            }
        }
    }

    info!("{message}");
    Ok(CaptureResult {
        path: Some(path_s),
        backend,
        message,
    })
}
