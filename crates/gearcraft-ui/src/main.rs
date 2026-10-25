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
    refresh_device_manager(&ui);

    let ui_weak = ui.as_weak();
    ui.on_refresh_devices(move || {
        if let Some(ui) = ui_weak.upgrade() {
            refresh(&ui);
            refresh_chatmix(&ui);
            refresh_audio_presets(&ui);
            refresh_device_manager(&ui);
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
            let bal = balance.clamp(-100, 100) as i8;
            match ipc::set_chatmix(bal) {
                Ok(v) => {
                    ui.set_chatmix_balance(bal as i32);
                    if let Some(g) = v.get("game_volume").and_then(|x| x.as_u64()) {
                        ui.set_game_volume(g as i32);
                    }
                    if let Some(c) = v.get("chat_volume").and_then(|x| x.as_u64()) {
                        ui.set_chat_volume(c as i32);
                    }
                    refresh_chatmix(&ui);
                }
                Err(e) => ui.set_status_text(SharedString::from(e.to_string())),
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
                    refresh_device_manager(&ui);
                }
                Err(e) => ui.set_status_text(SharedString::from(e.to_string())),
            }
        }
    });

    let ui_weak = ui.as_weak();
    ui.on_channel_volume(move |id, vol| {
        if let Some(ui) = ui_weak.upgrade() {
            update_channel(&ui, id.as_str(), |ch| ch.volume = vol.clamp(0, 100));
        }
    });

    let ui_weak = ui.as_weak();
    ui.on_channel_mute(move |id| {
        if let Some(ui) = ui_weak.upgrade() {
            update_channel(&ui, id.as_str(), |ch| ch.muted = !ch.muted);
        }
    });

    let ui_weak = ui.as_weak();
    ui.on_channel_solo(move |id| {
        if let Some(ui) = ui_weak.upgrade() {
            update_channel(&ui, id.as_str(), |ch| ch.solo = !ch.solo);
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
                    ui.set_status_text(SharedString::from(format!("Preset loaded: {name}")));
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
                    ui.set_status_text(SharedString::from(format!("Saved preset {name}")));
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
                    ui.set_status_text(SharedString::from("Preset deleted"));
                    refresh_audio_presets(&ui);
                }
                Err(e) => ui.set_status_text(SharedString::from(e.to_string())),
            }
        }
    });

    let ui_weak = ui.as_weak();
    ui.on_refresh_device_manager(move || {
        if let Some(ui) = ui_weak.upgrade() {
            refresh_device_manager(&ui);
        }
    });

    let ui_weak = ui.as_weak();
    ui.on_select_output_device(move |id| {
        if let Some(ui) = ui_weak.upgrade() {
            ui.set_selected_output_id(id.clone());
            let model = ui.get_output_devices();
            let mut label = SharedString::from("—");
            for i in 0..model.row_count() {
                if let Some(row) = model.row_data(i) {
                    if row.id == id {
                        label = row.label;
                        break;
                    }
                }
            }
            ui.set_selected_output_label(label);
        }
    });

    let ui_weak = ui.as_weak();
    ui.on_assign_channel_device(move |channel| {
        if let Some(ui) = ui_weak.upgrade() {
            let dev_id = ui.get_selected_output_id().to_string();
            if dev_id.is_empty() {
                ui.set_status_text(SharedString::from("Select an output device first"));
                return;
            }
            let ch = channel.to_string();
            let channel_id = match ch.as_str() {
                "game" => gearcraft_core::SonarChannelId::Game,
                "chat" => gearcraft_core::SonarChannelId::Chat,
                "media" => gearcraft_core::SonarChannelId::Media,
                "aux" => gearcraft_core::SonarChannelId::Aux,
                _ => {
                    ui.set_status_text(SharedString::from("Unknown channel"));
                    return;
                }
            };
            let policy = gearcraft_core::ChannelDevicePolicy {
                channel: channel_id,
                priority: vec![dev_id],
                excluded: vec![],
                active_device_id: None,
            };
            match ipc::set_channel_device_policy(policy) {
                Ok(_) => {
                    ui.set_status_text(SharedString::from(format!("Assigned output to {ch}")));
                    refresh_device_manager(&ui);
                }
                Err(e) => ui.set_status_text(SharedString::from(e.to_string())),
            }
        }
    });

    let ui_weak = ui.as_weak();
    ui.on_link_all_outputs(move || {
        if let Some(ui) = ui_weak.upgrade() {
            let enabled = !ui.get_link_all_devices();
            let dev = ui.get_selected_output_id().to_string();
            let device_id = if dev.is_empty() { None } else { Some(dev) };
            match ipc::link_all_devices(device_id, enabled) {
                Ok(_) => {
                    ui.set_link_all_devices(enabled);
                    ui.set_status_text(SharedString::from(if enabled {
                        "Link all channels: ON"
                    } else {
                        "Link all channels: OFF"
                    }));
                    refresh_device_manager(&ui);
                }
                Err(e) => ui.set_status_text(SharedString::from(e.to_string())),
            }
        }
    });

    let ui_weak = ui.as_weak();
    ui.on_apply_device_manager(move || {
        if let Some(ui) = ui_weak.upgrade() {
            match ipc::apply_device_manager() {
                Ok(_) => {
                    ui.set_status_text(SharedString::from("Device Manager applied"));
                    refresh_device_manager(&ui);
                }
                Err(e) => ui.set_status_text(SharedString::from(e.to_string())),
            }
        }
    });

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
                let id = item.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
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
                rows.push(DeviceRow {
                    id: SharedString::from(id),
                    name: SharedString::from(name),
                    vendor: SharedString::from(vendor),
                    kind: SharedString::from(kind),
                    online: true,
                    detail: SharedString::from(""),
                });
            }
            ui.set_devices(ModelRc::new(VecModel::from(rows)));
        }
        Ok(_) => ui.set_devices(ModelRc::new(VecModel::from(vec![]))),
        Err(e) => ui.set_status_text(SharedString::from(e.to_string())),
    }
}

fn refresh_chatmix(ui: &App) {
    match ipc::get_chatmix() {
        Ok(v) => {
            let balance = v.get("balance").and_then(|x| x.as_i64()).unwrap_or(0) as i32;
            let game = v.get("game_volume").and_then(|x| x.as_u64()).unwrap_or(100) as i32;
            let chat = v.get("chat_volume").and_then(|x| x.as_u64()).unwrap_or(100) as i32;
            ui.set_chatmix_balance(balance);
            ui.set_game_volume(game);
            ui.set_chat_volume(chat);
            ui.set_chatmix_ready(true);
            ui.set_chatmix_backend(SharedString::from("pipewire"));
            update_channel(ui, "game", |ch| ch.volume = game.clamp(0, 100));
            update_channel(ui, "chat", |ch| ch.volume = chat.clamp(0, 100));
        }
        Err(_) => {
            ui.set_chatmix_ready(false);
        }
    }
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

fn refresh_audio_presets(ui: &App) {
    match ipc::list_audio_presets() {
        Ok(Value::Array(arr)) => {
            let mut rows = Vec::new();
            for item in arr {
                let id = item.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
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

fn refresh_device_manager(ui: &App) {
    match ipc::get_device_manager() {
        Ok(data) => {
            let devices = data.get("devices").cloned().unwrap_or(Value::Array(vec![]));
            let mut rows = Vec::new();
            if let Value::Array(arr) = devices {
                for item in arr {
                    let id = item.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let desc = item
                        .get("description")
                        .and_then(|v| v.as_str())
                        .or_else(|| item.get("name").and_then(|v| v.as_str()))
                        .unwrap_or(&id)
                        .to_string();
                    let is_virtual = item
                        .get("is_virtual")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    let available = item
                        .get("available")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(true);
                    rows.push(OutputDeviceRow {
                        id: SharedString::from(id),
                        label: SharedString::from(desc),
                        is_virtual,
                        available,
                    });
                }
            }
            ui.set_output_devices(ModelRc::new(VecModel::from(rows)));

            if let Some(state) = data.get("state") {
                let link = state
                    .get("link_all")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true);
                ui.set_link_all_devices(link);
                let backup = state
                    .get("backup_mode")
                    .and_then(|v| v.as_str())
                    .unwrap_or("automatic");
                ui.set_backup_mode_label(SharedString::from(backup));

                let mut game = "—".to_string();
                let mut chat = "—".to_string();
                let mut media = "—".to_string();
                let mut aux = "—".to_string();
                if let Some(Value::Array(channels)) = state.get("channels") {
                    for ch in channels {
                        let name = ch.get("channel").and_then(|v| v.as_str()).unwrap_or("");
                        let active = ch
                            .get("active_device_id")
                            .and_then(|v| v.as_str())
                            .or_else(|| {
                                ch.get("priority")
                                    .and_then(|p| p.as_array())
                                    .and_then(|a| a.first())
                                    .and_then(|v| v.as_str())
                            })
                            .unwrap_or("—");
                        let short = if active.len() > 36 {
                            format!("…{}", &active[active.len() - 32..])
                        } else {
                            active.to_string()
                        };
                        match name {
                            "game" => game = short,
                            "chat" => chat = short,
                            "media" => media = short,
                            "aux" => aux = short,
                            _ => {}
                        }
                    }
                }
                ui.set_ch_device_game(SharedString::from(game));
                ui.set_ch_device_chat(SharedString::from(chat));
                ui.set_ch_device_media(SharedString::from(media));
                ui.set_ch_device_aux(SharedString::from(aux));
            }
        }
        Err(e) => {
            ui.set_status_text(SharedString::from(format!("Device Manager: {e}")));
        }
    }
}
