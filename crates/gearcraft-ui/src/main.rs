//! GearCraft UI — Slint desktop frontend.

mod ipc;

use serde_json::Value;
use slint::{ModelRc, SharedString, VecModel};
use tracing::info;

slint::include_modules!();

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let ui = App::new()?;
    refresh(&ui);
    refresh_chatmix(&ui);
    refresh_audio_presets(&ui);

    let ui_weak = ui.as_weak();
    ui.on_refresh_devices(move || {
        if let Some(ui) = ui_weak.upgrade() {
            refresh(&ui);
            refresh_chatmix(&ui);
            refresh_audio_presets(&ui);
        }
    });

    let ui_weak = ui.as_weak();
    ui.on_select_device(move |id| {
        if let Some(ui) = ui_weak.upgrade() {
            ui.set_selected_device_id(id.clone());
            let devices = ui.get_devices();
            let mut name = SharedString::from("—");
            for i in 0..devices.row_count() {
                if let Some(row) = devices.row_data(i) {
                    if row.id == id {
                        name = row.name;
                        break;
                    }
                }
            }
            ui.set_selected_device_name(name);
        }
    });

    let ui_weak = ui.as_weak();
    ui.on_apply_color(move || {
        if let Some(ui) = ui_weak.upgrade() {
            let id = ui.get_selected_device_id();
            if id.is_empty() {
                ui.set_status_text(SharedString::from("Select a device first"));
                return;
            }
            let zone = ui.get_color_zone().to_string();
            let r = (ui.get_color_r() * 255.0).round().clamp(0.0, 255.0) as u8;
            let g = (ui.get_color_g() * 255.0).round().clamp(0.0, 255.0) as u8;
            let b = (ui.get_color_b() * 255.0).round().clamp(0.0, 255.0) as u8;
            match ipc::set_color(id.as_str(), &zone, r, g, b) {
                Ok(()) => ui.set_status_text(SharedString::from(format!(
                    "Colour applied ({r},{g},{b}) on {zone}"
                ))),
                Err(e) => ui.set_status_text(SharedString::from(e.to_string())),
            }
        }
    });

    let ui_weak = ui.as_weak();
    ui.on_set_chatmix(move |balance| {
        if let Some(ui) = ui_weak.upgrade() {
            if !ui.get_sonar_enabled() {
                return;
            }
            let b = balance.clamp(-100, 100) as i8;
            match ipc::set_chatmix(b) {
                Ok(v) => apply_chatmix_json(&ui, &v),
                Err(e) => {
                    ui.set_chatmix_balance(b as i32);
                    ui.set_game_volume(((100 + b as i32) / 2).clamp(0, 100));
                    ui.set_chat_volume(((100 - b as i32) / 2).clamp(0, 100));
                    ui.set_status_text(SharedString::from(e.to_string()));
                }
            }
        }
    });

    let ui_weak = ui.as_weak();
    ui.on_setup_chatmix(move || {
        if let Some(ui) = ui_weak.upgrade() {
            match ipc::setup_chatmix() {
                Ok(v) => {
                    let msg = v
                        .get("message")
                        .and_then(|x| x.as_str())
                        .unwrap_or("sinks ready");
                    ui.set_status_text(SharedString::from(msg));
                    refresh_chatmix(&ui);
                }
                Err(e) => ui.set_status_text(SharedString::from(e.to_string())),
            }
        }
    });

    let ui_weak = ui.as_weak();
    ui.on_channel_volume(move |id, vol| {
        if let Some(ui) = ui_weak.upgrade() {
            update_channel(&ui, id.as_str(), |ch| {
                ch.volume = vol;
            });
            if id == "game" || id == "chat" {
                let (game, chat) = channel_volumes(&ui);
                let sum = game + chat;
                let balance = if sum == 0 {
                    0
                } else {
                    (((game - chat) as f32 / sum as f32) * 100.0).round() as i32
                };
                let b = balance.clamp(-100, 100) as i8;
                let _ = ipc::set_chatmix(b).map(|v| apply_chatmix_json(&ui, &v));
            }
        }
    });

    let ui_weak = ui.as_weak();
    ui.on_channel_mute(move |id| {
        if let Some(ui) = ui_weak.upgrade() {
            update_channel(&ui, id.as_str(), |ch| {
                ch.muted = !ch.muted;
            });
        }
    });

    let ui_weak = ui.as_weak();
    ui.on_channel_solo(move |id| {
        if let Some(ui) = ui_weak.upgrade() {
            update_channel(&ui, id.as_str(), |ch| {
                ch.solo = !ch.solo;
            });
        }
    });

    let ui_weak = ui.as_weak();
    ui.on_toggle_streamer(move || {
        if let Some(ui) = ui_weak.upgrade() {
            ui.set_streamer_mode(!ui.get_streamer_mode());
        }
    });

    let ui_weak = ui.as_weak();
    ui.on_toggle_sonar(move || {
        if let Some(ui) = ui_weak.upgrade() {
            ui.set_sonar_enabled(!ui.get_sonar_enabled());
        }
    });

    let ui_weak = ui.as_weak();
    ui.on_refresh_presets(move || {
        if let Some(ui) = ui_weak.upgrade() {
            refresh_audio_presets(&ui);
        }
    });

    let ui_weak = ui.as_weak();
    ui.on_load_preset(move |id| {
        if let Some(ui) = ui_weak.upgrade() {
            match ipc::load_audio_preset(id.as_str()) {
                Ok(v) => {
                    let name = v
                        .get("preset_name")
                        .and_then(|x| x.as_str())
                        .unwrap_or("preset");
                    ui.set_active_preset_id(id.clone());
                    ui.set_active_preset_name(SharedString::from(name));
                    let dsp = v
                        .get("dsp_message")
                        .and_then(|x| x.as_str())
                        .unwrap_or("");
                    let msg = if dsp.is_empty() {
                        format!("Preset loaded: {name}")
                    } else {
                        format!("Preset «{name}» — {dsp}")
                    };
                    ui.set_status_text(SharedString::from(msg));
                    refresh_chatmix(&ui);
                    refresh_audio_presets(&ui);
                }
                Err(e) => ui.set_status_text(SharedString::from(e.to_string())),
            }
        }
    });

    let ui_weak = ui.as_weak();
    ui.on_save_preset(move |name| {
        if let Some(ui) = ui_weak.upgrade() {
            let name = name.to_string();
            if name.trim().is_empty() {
                ui.set_status_text(SharedString::from("Preset name required"));
                return;
            }
            let id = name
                .to_lowercase()
                .chars()
                .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
                .collect::<String>();
            let balance = ui.get_chatmix_balance() as i8;
            let game = ui.get_game_volume() as u8;
            let chat = ui.get_chat_volume() as u8;
            let preset = gearcraft_core::AudioPreset {
                id: id.clone(),
                name: name.clone(),
                effects: gearcraft_core::AudioEffects::default(),
                chatmix: Some(gearcraft_core::ChatMixState {
                    balance,
                    game_volume: game,
                    chat_volume: chat,
                }),
                notes: String::new(),
            };
            match ipc::save_audio_preset(preset) {
                Ok(_) => {
                    ui.set_active_preset_id(SharedString::from(id));
                    ui.set_active_preset_name(SharedString::from(name.clone()));
                    ui.set_status_text(SharedString::from(format!("Preset saved: {name}")));
                    refresh_audio_presets(&ui);
                }
                Err(e) => ui.set_status_text(SharedString::from(e.to_string())),
            }
        }
    });

    let ui_weak = ui.as_weak();
    ui.on_delete_preset(move |id| {
        if let Some(ui) = ui_weak.upgrade() {
            match ipc::delete_audio_preset(id.as_str()) {
                Ok(_) => {
                    if ui.get_active_preset_id() == id {
                        ui.set_active_preset_id(SharedString::from(""));
                        ui.set_active_preset_name(SharedString::from("—"));
                    }
                    ui.set_status_text(SharedString::from("Preset deleted"));
                    refresh_audio_presets(&ui);
                }
                Err(e) => ui.set_status_text(SharedString::from(e.to_string())),
            }
        }
    });

    info!("GearCraft UI (Slint) starting");
    ui.run()?;
    Ok(())
}

fn refresh(ui: &App) {
    match ipc::ping() {
        Ok(()) => {
            ui.set_daemon_online(true);
            ui.set_status_text(SharedString::from("Daemon online"));
        }
        Err(e) => {
            ui.set_daemon_online(false);
            ui.set_status_text(SharedString::from(e.to_string()));
            ui.set_devices(ModelRc::new(VecModel::from(vec![])));
            return;
        }
    }

    match ipc::get_devices() {
        Ok(Value::Array(arr)) => {
            let mut rows = Vec::new();
            for item in arr {
                let id = item
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let name = item
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Device")
                    .to_string();
                let vendor = item
                    .get("vendor")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let kind = item
                    .get("kind")
                    .and_then(|v| v.as_str())
                    .unwrap_or("other")
                    .to_string();
                let path = item
                    .get("path")
                    .and_then(|v| v.as_str())
                    .unwrap_or("—");
                rows.push(DeviceRow {
                    id: id.into(),
                    name: name.into(),
                    vendor: vendor.into(),
                    kind: kind.into(),
                    online: true,
                    detail: path.into(),
                });
            }
            if ui.get_selected_device_id().is_empty() {
                if let Some(first) = rows.first() {
                    ui.set_selected_device_id(first.id.clone());
                    ui.set_selected_device_name(first.name.clone());
                }
            }
            ui.set_devices(ModelRc::new(VecModel::from(rows)));
        }
        Ok(_) => ui.set_devices(ModelRc::new(VecModel::from(vec![]))),
        Err(e) => ui.set_status_text(SharedString::from(e.to_string())),
    }
}

fn refresh_chatmix(ui: &App) {
    if let Ok(v) = ipc::get_chatmix() {
        apply_chatmix_json(ui, &v);
    }
}

fn apply_chatmix_json(ui: &App, v: &serde_json::Value) {
    let balance = v.get("balance").and_then(|x| x.as_i64()).unwrap_or(0) as i32;
    let game = v
        .get("game_volume")
        .and_then(|x| x.as_u64().or_else(|| x.as_f64().map(|f| f as u64)))
        .unwrap_or(100) as i32;
    let chat = v
        .get("chat_volume")
        .and_then(|x| x.as_u64().or_else(|| x.as_f64().map(|f| f as u64)))
        .unwrap_or(100) as i32;
    let backend = v.get("backend").and_then(|x| x.as_str()).unwrap_or("—");
    let ready = v.get("available").and_then(|x| x.as_bool()).unwrap_or(false);

    ui.set_chatmix_balance(balance);
    ui.set_game_volume(game.clamp(0, 100));
    ui.set_chat_volume(chat.clamp(0, 100));
    ui.set_chatmix_backend(SharedString::from(backend));
    ui.set_chatmix_ready(ready);

    update_channel(ui, "game", |ch| ch.volume = game.clamp(0, 100));
    update_channel(ui, "chat", |ch| ch.volume = chat.clamp(0, 100));
}

fn update_channel(ui: &App, id: &str, f: impl FnOnce(&mut ChannelRow)) {
    let model = ui.get_channels();
    let mut rows = Vec::new();
    for i in 0..model.row_count() {
        if let Some(mut row) = model.row_data(i) {
            if row.id.as_str() == id {
                f(&mut row);
            }
            rows.push(row);
        }
    }
    ui.set_channels(ModelRc::new(VecModel::from(rows)));
}

fn channel_volumes(ui: &App) -> (i32, i32) {
    let model = ui.get_channels();
    let mut game = 100;
    let mut chat = 100;
    for i in 0..model.row_count() {
        if let Some(row) = model.row_data(i) {
            if row.id == "game" {
                game = row.volume;
            }
            if row.id == "chat" {
                chat = row.volume;
            }
        }
    }
    (game, chat)
}

fn refresh_audio_presets(ui: &App) {
    match ipc::list_audio_presets() {
        Ok(Value::Array(arr)) => {
            let mut rows = Vec::new();
            for item in arr {
                let id = item
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let name = item
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Preset")
                    .to_string();
                let notes = item
                    .get("notes")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let eq = item
                    .get("effects")
                    .and_then(|e| e.get("eq_enabled"))
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                rows.push(PresetRow {
                    id: SharedString::from(id),
                    name: SharedString::from(name),
                    notes: SharedString::from(notes),
                    eq_enabled: eq,
                });
            }
            ui.set_presets(ModelRc::new(VecModel::from(rows)));
        }
        Ok(_) => ui.set_presets(ModelRc::new(VecModel::from(vec![]))),
        Err(e) => ui.set_status_text(SharedString::from(e.to_string())),
    }
}
