//! Camera enumeration, settings, effects, and live V4L2 preview.

use std::fs;
use std::io::Cursor;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CameraDevice {
    pub id: String,
    pub label: String,
    pub path: String,
    pub available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CameraEffect {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub enabled: bool,
    pub intensity: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CameraSettings {
    pub device_id: String,
    pub device_path: String,
    pub video_resolution: String,
    pub video_fps: u32,
    pub video_format: String,
    pub video_bitrate_mbps: u32,
    pub photo_resolution: String,
    pub photo_format: String,
    pub photo_quality: u8,
    pub photo_timer_s: u32,
    pub brightness: u8,
    pub contrast: u8,
    pub saturation: u8,
    pub sharpness: u8,
    pub exposure: u8,
    pub auto_exposure: bool,
    pub auto_wb: bool,
    pub wb_temp: u32,
    pub effects: Vec<CameraEffect>,
}

impl Default for CameraSettings {
    fn default() -> Self {
        Self {
            device_id: String::new(),
            device_path: String::new(),
            video_resolution: "1280x720".into(),
            video_fps: 30,
            video_format: "MJPG".into(),
            video_bitrate_mbps: 8,
            photo_resolution: "1920x1080".into(),
            photo_format: "JPEG".into(),
            photo_quality: 90,
            photo_timer_s: 0,
            brightness: 50,
            contrast: 50,
            saturation: 50,
            sharpness: 50,
            exposure: 50,
            auto_exposure: true,
            auto_wb: true,
            wb_temp: 4500,
            effects: Vec::new(),
        }
    }
}

#[derive(Clone)]
pub struct PreviewFrame {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

struct PreviewState {
    stop: AtomicBool,
    active: AtomicBool,
    recording: AtomicBool,
    frame: Mutex<Option<PreviewFrame>>,
    status: Mutex<String>,
    join: Mutex<Option<JoinHandle<()>>>,
}

static PREVIEW: Lazy<PreviewState> = Lazy::new(|| PreviewState {
    stop: AtomicBool::new(true),
    active: AtomicBool::new(false),
    recording: AtomicBool::new(false),
    frame: Mutex::new(None),
    status: Mutex::new("Preview spenta".into()),
    join: Mutex::new(None),
});

static EFFECT_SEQ: AtomicU64 = AtomicU64::new(1);
static SETTINGS: Lazy<Mutex<CameraSettings>> =
    Lazy::new(|| Mutex::new(CameraSettings::default()));

pub fn list_devices() -> Vec<CameraDevice> {
    let mut out = Vec::new();
    let Ok(entries) = fs::read_dir("/dev") else {
        return out;
    };
    let mut paths: Vec<_> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(|n| {
                    n.starts_with("video")
                        && n.as_bytes().get(5..).map_or(false, |rest| {
                            !rest.is_empty() && rest.iter().all(|c| c.is_ascii_digit())
                        })
                })
                .unwrap_or(false)
        })
        .collect();
    paths.sort();
    for (i, path) in paths.into_iter().enumerate() {
        let path_s = path.display().to_string();
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("video");
        out.push(CameraDevice {
            id: format!("cam-{i}"),
            label: format!("{name} ({path_s})"),
            path: path_s,
            available: true,
        });
    }
    out
}

pub fn settings() -> CameraSettings {
    SETTINGS.lock().unwrap().clone()
}

pub fn update_settings(f: impl FnOnce(&mut CameraSettings)) {
    let mut s = SETTINGS.lock().unwrap();
    f(&mut s);
}

pub fn select_device(id: &str, path: &str) {
    update_settings(|s| {
        s.device_id = id.to_string();
        s.device_path = path.to_string();
    });
}

pub fn preview_active() -> bool {
    PREVIEW.active.load(Ordering::Relaxed)
}

pub fn recording() -> bool {
    PREVIEW.recording.load(Ordering::Relaxed)
}

pub fn preview_status() -> String {
    PREVIEW.status.lock().unwrap().clone()
}

fn set_status(msg: impl Into<String>) {
    *PREVIEW.status.lock().unwrap() = msg.into();
}

pub fn take_frame() -> Option<PreviewFrame> {
    PREVIEW.frame.lock().unwrap().clone()
}

pub fn set_preview(on: bool) -> String {
    if on {
        start_preview()
    } else {
        stop_preview();
        set_status("Preview spenta");
        "Preview spenta".into()
    }
}

fn stop_preview() {
    PREVIEW.stop.store(true, Ordering::SeqCst);
    PREVIEW.active.store(false, Ordering::SeqCst);
    PREVIEW.recording.store(false, Ordering::SeqCst);
    if let Some(h) = PREVIEW.join.lock().unwrap().take() {
        let _ = h.join();
    }
}

fn start_preview() -> String {
    let mut s = settings();
    if s.device_path.is_empty() {
        let devices = list_devices();
        let path = devices
            .iter()
            .find(|d| d.id == s.device_id)
            .map(|d| d.path.clone())
            .unwrap_or_default();
        if path.is_empty() {
            return "Seleziona una camera prima".into();
        }
        update_settings(|st| st.device_path = path);
        s = settings();
    }
    if s.device_path.is_empty() {
        return "Seleziona una camera prima".into();
    }

    stop_preview();
    PREVIEW.stop.store(false, Ordering::SeqCst);
    PREVIEW.active.store(true, Ordering::SeqCst);

    let path = s.device_path.clone();
    let (req_w, req_h) = parse_resolution(&s.video_resolution);
    let prefer_mjpg = s.video_format.eq_ignore_ascii_case("MJPG")
        || s.video_format.eq_ignore_ascii_case("JPEG");
    let fps = s.video_fps.max(1);

    let handle = thread::Builder::new()
        .name("gearcraft-cam".into())
        .spawn(move || capture_loop(path, req_w, req_h, prefer_mjpg, fps))
        .expect("spawn camera thread");
    *PREVIEW.join.lock().unwrap() = Some(handle);

    set_status(format!(
        "Preview su {} · {} @ {} fps",
        s.device_path, s.video_resolution, s.video_fps
    ));
    preview_status()
}

fn parse_resolution(s: &str) -> (u32, u32) {
    let mut parts = s.split('x');
    let w = parts.next().and_then(|x| x.parse().ok()).unwrap_or(1280);
    let h = parts.next().and_then(|x| x.parse().ok()).unwrap_or(720);
    (w.max(160), h.max(120))
}

fn capture_loop(path: String, req_w: u32, req_h: u32, prefer_mjpg: bool, fps: u32) {
    match open_and_stream(&path, req_w, req_h, prefer_mjpg, fps) {
        Ok(()) => {}
        Err(e) => {
            tracing::warn!("V4L2 capture failed ({e}); using test pattern");
            set_status(format!("V4L2: {e} — pattern di test"));
            test_pattern_loop(req_w, req_h, fps);
        }
    }
    PREVIEW.active.store(false, Ordering::SeqCst);
}

fn open_and_stream(
    path: &str,
    req_w: u32,
    req_h: u32,
    prefer_mjpg: bool,
    fps: u32,
) -> Result<(), String> {
    use v4l::buffer::Type;
    use v4l::io::traits::CaptureStream;
    use v4l::video::Capture;
    use v4l::{Device, FourCC};

    if !Path::new(path).exists() {
        return Err(format!("{path} non esiste"));
    }

    let dev = Device::with_path(path).map_err(|e| format!("open {path}: {e}"))?;
    let mut fmt = dev.format().map_err(|e| format!("format: {e}"))?;
    fmt.width = req_w;
    fmt.height = req_h;
    fmt.fourcc = if prefer_mjpg {
        FourCC::new(b"MJPG")
    } else {
        FourCC::new(b"YUYV")
    };
    let fmt = dev.set_format(&fmt).map_err(|e| format!("set_format: {e}"))?;
    let width = fmt.width;
    let height = fmt.height;
    let fourcc = fmt.fourcc;
    tracing::info!("camera {path} format {width}x{height} {:?}", fourcc);

    let mut stream = v4l::io::mmap::Stream::with_buffers(&dev, Type::VideoCapture, 4)
        .map_err(|e| format!("mmap stream: {e}"))?;

    set_status(format!(
        "LIVE {}×{} · {}",
        width,
        height,
        String::from_utf8_lossy(&fourcc.repr)
    ));

    let frame_delay = Duration::from_micros(1_000_000 / u64::from(fps.max(1)));

    while !PREVIEW.stop.load(Ordering::Relaxed) {
        let (buf, _meta) = stream.next().map_err(|e| format!("capture: {e}"))?;
        let rgba = if fourcc == FourCC::new(b"MJPG") || fourcc == FourCC::new(b"JPEG") {
            decode_jpeg_to_rgba(buf)?
        } else if fourcc == FourCC::new(b"YUYV") || fourcc == FourCC::new(b"YUY2") {
            yuyv_to_rgba(buf, width, height)
        } else {
            decode_jpeg_to_rgba(buf).unwrap_or_else(|_| yuyv_to_rgba(buf, width, height))
        };

        let pixels = (rgba.len() / 4) as u32;
        let (w, h) = if pixels >= width * height {
            (width, height)
        } else if width > 0 {
            (width, (pixels / width).max(1))
        } else {
            (1, 1)
        };

        let mut frame = PreviewFrame {
            width: w,
            height: h,
            rgba,
        };
        apply_effects_inplace(&mut frame);
        *PREVIEW.frame.lock().unwrap() = Some(frame);
        thread::sleep(frame_delay);
    }
    Ok(())
}

fn decode_jpeg_to_rgba(data: &[u8]) -> Result<Vec<u8>, String> {
    let mut decoder = jpeg_decoder::Decoder::new(Cursor::new(data));
    let pixels = decoder.decode().map_err(|e| format!("jpeg: {e}"))?;
    let info = decoder.info().ok_or_else(|| "jpeg info".to_string())?;
    let mut rgba = Vec::with_capacity(pixels.len() * 2);
    match info.pixel_format {
        jpeg_decoder::PixelFormat::RGB24 => {
            for chunk in pixels.chunks(3) {
                rgba.extend_from_slice(&[chunk[0], chunk[1], chunk.get(2).copied().unwrap_or(0), 255]);
            }
        }
        jpeg_decoder::PixelFormat::L8 => {
            for &y in &pixels {
                rgba.extend_from_slice(&[y, y, y, 255]);
            }
        }
        jpeg_decoder::PixelFormat::CMYK32 => {
            for chunk in pixels.chunks(4) {
                let c = chunk[0] as f32 / 255.0;
                let m = chunk[1] as f32 / 255.0;
                let y = chunk[2] as f32 / 255.0;
                let k = chunk[3] as f32 / 255.0;
                rgba.push(((1.0 - c) * (1.0 - k) * 255.0) as u8);
                rgba.push(((1.0 - m) * (1.0 - k) * 255.0) as u8);
                rgba.push(((1.0 - y) * (1.0 - k) * 255.0) as u8);
                rgba.push(255);
            }
        }
        _ => {
            for chunk in pixels.chunks(3) {
                if chunk.len() >= 3 {
                    rgba.extend_from_slice(&[chunk[0], chunk[1], chunk[2], 255]);
                }
            }
        }
    }
    Ok(rgba)
}

fn yuyv_to_rgba(data: &[u8], width: u32, height: u32) -> Vec<u8> {
    let w = width as usize;
    let h = height as usize;
    let mut out = vec![0u8; w * h * 4];
    let mut i = 0;
    let mut o = 0;
    while o + 7 < out.len() && i + 3 < data.len() {
        let y0 = data[i] as i32;
        let u = data[i + 1] as i32;
        let y1 = data[i + 2] as i32;
        let v = data[i + 3] as i32;
        let (r0, g0, b0) = yuv_to_rgb(y0, u, v);
        let (r1, g1, b1) = yuv_to_rgb(y1, u, v);
        out[o] = r0;
        out[o + 1] = g0;
        out[o + 2] = b0;
        out[o + 3] = 255;
        out[o + 4] = r1;
        out[o + 5] = g1;
        out[o + 6] = b1;
        out[o + 7] = 255;
        i += 4;
        o += 8;
    }
    let _ = h;
    out
}

fn yuv_to_rgb(y: i32, u: i32, v: i32) -> (u8, u8, u8) {
    let c = y - 16;
    let d = u - 128;
    let e = v - 128;
    let r = (298 * c + 409 * e + 128) >> 8;
    let g = (298 * c - 100 * d - 208 * e + 128) >> 8;
    let b = (298 * c + 516 * d + 128) >> 8;
    (
        r.clamp(0, 255) as u8,
        g.clamp(0, 255) as u8,
        b.clamp(0, 255) as u8,
    )
}

fn apply_effects_inplace(frame: &mut PreviewFrame) {
    let effects = settings().effects;
    for fx in effects.iter().filter(|e| e.enabled) {
        let t = fx.intensity as f32 / 100.0;
        match fx.kind.as_str() {
            "mirror" => mirror_h(frame),
            "vignette" => vignette(frame, t),
            "blur" => box_blur(frame, 1 + (t * 3.0) as usize),
            "sharpen" => sharpen(frame, t),
            "color_grade" => color_grade(frame, t),
            _ => {}
        }
    }
    let s = settings();
    let bright = (s.brightness as i16 - 50) * 2;
    let contrast = s.contrast as f32 / 50.0;
    for px in frame.rgba.chunks_exact_mut(4) {
        for c in 0..3 {
            let mut v = px[c] as f32;
            v = (v - 128.0) * contrast + 128.0 + bright as f32;
            px[c] = v.clamp(0.0, 255.0) as u8;
        }
    }
}

fn mirror_h(frame: &mut PreviewFrame) {
    let w = frame.width as usize;
    let h = frame.height as usize;
    if w == 0 {
        return;
    }
    for y in 0..h {
        let row = y * w * 4;
        for x in 0..(w / 2) {
            let a = row + x * 4;
            let b = row + (w - 1 - x) * 4;
            for k in 0..4 {
                frame.rgba.swap(a + k, b + k);
            }
        }
    }
}

fn vignette(frame: &mut PreviewFrame, strength: f32) {
    let w = frame.width as f32;
    let h = frame.height as f32;
    let cx = w / 2.0;
    let cy = h / 2.0;
    let max_d = (cx * cx + cy * cy).sqrt().max(1.0);
    for y in 0..frame.height {
        for x in 0..frame.width {
            let dx = x as f32 - cx;
            let dy = y as f32 - cy;
            let d = (dx * dx + dy * dy).sqrt() / max_d;
            let factor = 1.0 - strength * d * d;
            let i = ((y * frame.width + x) * 4) as usize;
            if i + 2 < frame.rgba.len() {
                for k in 0..3 {
                    frame.rgba[i + k] =
                        ((frame.rgba[i + k] as f32) * factor).clamp(0.0, 255.0) as u8;
                }
            }
        }
    }
}

fn box_blur(frame: &mut PreviewFrame, radius: usize) {
    if radius == 0 {
        return;
    }
    let w = frame.width as usize;
    let h = frame.height as usize;
    let src = frame.rgba.clone();
    for y in 0..h {
        for x in 0..w {
            let mut r = 0u32;
            let mut g = 0u32;
            let mut b = 0u32;
            let mut n = 0u32;
            for dy in 0..=radius {
                let yy = (y + dy).min(h.saturating_sub(1));
                for dx in 0..=radius {
                    let xx = (x + dx).min(w.saturating_sub(1));
                    let i = (yy * w + xx) * 4;
                    r += src[i] as u32;
                    g += src[i + 1] as u32;
                    b += src[i + 2] as u32;
                    n += 1;
                }
            }
            let o = (y * w + x) * 4;
            frame.rgba[o] = (r / n) as u8;
            frame.rgba[o + 1] = (g / n) as u8;
            frame.rgba[o + 2] = (b / n) as u8;
        }
    }
}

fn sharpen(frame: &mut PreviewFrame, amount: f32) {
    let w = frame.width as usize;
    let h = frame.height as usize;
    if w < 3 || h < 3 {
        return;
    }
    let src = frame.rgba.clone();
    for y in 1..h - 1 {
        for x in 1..w - 1 {
            let i = (y * w + x) * 4;
            for c in 0..3 {
                let center = src[i + c] as f32;
                let up = src[i - w * 4 + c] as f32;
                let down = src[i + w * 4 + c] as f32;
                let left = src[i - 4 + c] as f32;
                let right = src[i + 4 + c] as f32;
                let lap = 5.0 * center - up - down - left - right;
                let v = center + amount * (lap - center);
                frame.rgba[i + c] = v.clamp(0.0, 255.0) as u8;
            }
        }
    }
}

fn color_grade(frame: &mut PreviewFrame, t: f32) {
    for px in frame.rgba.chunks_exact_mut(4) {
        let r = px[0] as f32;
        let g = px[1] as f32;
        let b = px[2] as f32;
        px[0] = (r * (1.0 + 0.15 * t) + 10.0 * t).clamp(0.0, 255.0) as u8;
        px[1] = (g * (1.0 - 0.05 * t)).clamp(0.0, 255.0) as u8;
        px[2] = (b * (1.0 + 0.1 * t) + 8.0 * t).clamp(0.0, 255.0) as u8;
    }
}

fn test_pattern_loop(width: u32, height: u32, fps: u32) {
    let frame_delay = Duration::from_micros(1_000_000 / u64::from(fps.max(1)));
    let mut t = 0u32;
    while !PREVIEW.stop.load(Ordering::Relaxed) {
        let mut rgba = vec![0u8; (width * height * 4) as usize];
        for y in 0..height {
            for x in 0..width {
                let i = ((y * width + x) * 4) as usize;
                let px = ((x + t) ^ (y + t / 2)) as u8;
                rgba[i] = px;
                rgba[i + 1] = px.wrapping_mul(3);
                rgba[i + 2] = 255u8.wrapping_sub(px);
                rgba[i + 3] = 255;
            }
        }
        let mut frame = PreviewFrame {
            width,
            height,
            rgba,
        };
        apply_effects_inplace(&mut frame);
        *PREVIEW.frame.lock().unwrap() = Some(frame);
        t = t.wrapping_add(3);
        thread::sleep(frame_delay);
    }
}

pub fn toggle_recording() -> (bool, String) {
    if !preview_active() {
        return (false, "Avvia la preview prima di registrare".into());
    }
    let next = !PREVIEW.recording.load(Ordering::Relaxed);
    PREVIEW.recording.store(next, Ordering::Relaxed);
    if next {
        let s = settings();
        (
            true,
            format!(
                "REC {} · {} Mbps (mux non ancora collegato)",
                s.video_resolution, s.video_bitrate_mbps
            ),
        )
    } else {
        (false, "Registrazione fermata".into())
    }
}

pub fn capture_photo() -> String {
    let s = settings();
    if s.device_id.is_empty() && s.device_path.is_empty() {
        return "Nessuna camera selezionata".into();
    }
    if let Some(frame) = take_frame() {
        let dir = dirs_photo();
        let _ = fs::create_dir_all(&dir);
        let path = dir.join(format!(
            "gearcraft-{}.ppm",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0)
        ));
        if write_ppm(&path, &frame).is_ok() {
            return format!("Foto salvata: {}", path.display());
        }
    }
    format!(
        "Foto {} · {} q{} (nessun frame — avvia preview)",
        s.photo_resolution, s.photo_format, s.photo_quality
    )
}

fn dirs_photo() -> std::path::PathBuf {
    if let Ok(home) = std::env::var("HOME") {
        Path::new(&home).join("Pictures").join("GearCraft")
    } else {
        Path::new("/tmp").join("gearcraft-photos")
    }
}

fn write_ppm(path: &Path, frame: &PreviewFrame) -> std::io::Result<()> {
    use std::io::Write;
    let mut f = fs::File::create(path)?;
    writeln!(f, "P6\n{} {}\n255", frame.width, frame.height)?;
    for px in frame.rgba.chunks_exact(4) {
        f.write_all(&[px[0], px[1], px[2]])?;
    }
    Ok(())
}

pub fn add_effect(kind: &str) -> CameraEffect {
    let id = format!("fx-{}", EFFECT_SEQ.fetch_add(1, Ordering::Relaxed));
    let name = match kind {
        "blur" => "Blur",
        "color_grade" => "Color grade",
        "vignette" => "Vignette",
        "mirror" => "Mirror",
        "overlay" => "Overlay",
        "sharpen" => "Sharpen",
        other => other,
    };
    let fx = CameraEffect {
        id: id.clone(),
        name: name.into(),
        kind: kind.into(),
        enabled: true,
        intensity: 50,
    };
    update_settings(|s| s.effects.push(fx.clone()));
    fx
}

pub fn remove_effect(id: &str) {
    update_settings(|s| s.effects.retain(|e| e.id != id));
}

pub fn toggle_effect(id: &str) {
    update_settings(|s| {
        if let Some(e) = s.effects.iter_mut().find(|e| e.id == id) {
            e.enabled = !e.enabled;
        }
    });
}

pub fn set_effect_intensity(id: &str, intensity: u8) {
    update_settings(|s| {
        if let Some(e) = s.effects.iter_mut().find(|e| e.id == id) {
            e.intensity = intensity.min(100);
        }
    });
}

pub fn apply_summary() -> String {
    let s = settings();
    let n_fx = s.effects.iter().filter(|e| e.enabled).count();
    format!(
        "Camera · {} · video {}@{} · {} effetti · preview={}",
        if s.device_path.is_empty() {
            "no-device"
        } else {
            s.device_path.as_str()
        },
        s.video_resolution,
        s.video_fps,
        n_fx,
        preview_active()
    )
}
