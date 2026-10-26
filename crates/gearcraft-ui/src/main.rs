//! GearCraft UI — Slint desktop frontend.

mod ipc;
mod camera;

use serde_json::Value;
use slint::{Image, Model, ModelRc, Rgba8Pixel, SharedPixelBuffer, SharedString, Timer, TimerMode, VecModel};
use tracing::info;

slint::include_modules!();


const EQ_FREQS: [f32; 10] = [31.5, 63.0, 125.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0, 16000.0];
const EQ_LABELS: [&str; 10] = ["31", "63", "125", "250", "500", "1k", "2k", "4k", "8k", "16k"];

fn default_eq_rows() -> Vec<EqBandRow> {
    EQ_LABELS
        .iter()
        .enumerate()
        .map(|(i, lab)| EqBandRow {
            index: i as i32,
            freq_label: SharedString::from(*lab),
            gain: 0.0,
        })
        .collect()
}

fn collect_eq_effects(ui: &App) -> gearcraft_core::AudioEffects {
    let mut fx = gearcraft_core::AudioEffects::default();
    fx.eq_enabled = ui.get_eq_enabled();
    fx.spatial_enabled = ui.get_spatial_enabled();
    fx.spatial_immersion = ui.get_spatial_immersion().clamp(0, 100) as u8;
    fx.spatial_distance = ui.get_spatial_distance().clamp(0, 100) as u8;
    fx.volume_boost_db = ui.get_volume_boost() as f32;
    fx.smart_volume = ui.get_smart_volume();
    fx.limiter = true;
    let model = ui.get_eq_bands();
    let mut bands = Vec::new();
    for i in 0..model.row_count().min(10) {
        if let Some(row) = model.row_data(i) {
            let gain = row.gain.clamp(-12.0, 12.0);
            bands.push(gearcraft_core::EqBand {
                freq_hz: EQ_FREQS[i],
                gain_db: gain,
            });
        }
    }
    if bands.is_empty() {
        bands = EQ_FREQS
            .iter()
            .map(|&freq_hz| gearcraft_core::EqBand {
                freq_hz,
                gain_db: 0.0,
            })
            .collect();
    }
    fx.eq_bands = bands;
    fx
}

fn set_eq_gain(ui: &App, index: i32, gain: f32) {
    let model = ui.get_eq_bands();
    let mut rows = Vec::new();
    for i in 0..model.row_count() {
        if let Some(mut row) = model.row_data(i) {
            if row.index == index {
                row.gain = gain.clamp(-12.0, 12.0);
            }
            rows.push(row);
        }
    }
    ui.set_eq_bands(ModelRc::new(VecModel::from(rows)));
}

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
    refresh_mic_fx(&ui);
    refresh_cameras(&ui);

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
            // Find name from current model
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
            refresh_audio_presets(&ui);
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
            // Drive ChatMix from game/chat faders
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

    let ui_weak = ui.as_weak();
    ui.on_toggle_preset_rule(move |preset_id| {
        if let Some(ui) = ui_weak.upgrade() {
            let device_id = ui.get_selected_device_id().to_string();
            if device_id.is_empty() {
                ui.set_status_text(SharedString::from("Select a device first"));
                return;
            }
            let preset_id = preset_id.to_string();
            let device_name = ui.get_selected_device_name().to_string();
            match auto_rule_for(&device_id) {
                Ok(Some(current)) if current == preset_id => {
                    match ipc::delete_preset_rule(&device_id, "") {
                        Ok(_) => ui.set_status_text(SharedString::from(format!(
                            "Auto-apply removed for {device_name}"
                        ))),
                        Err(e) => ui.set_status_text(SharedString::from(e.to_string())),
                    }
                }
                Ok(_) => {
                    let rule = gearcraft_core::PresetRule {
                        device_id: device_id.clone(),
                        device_name,
                        route_id: String::new(),
                        preset_id: preset_id.clone(),
                    };
                    match ipc::set_preset_rule(rule) {
                        Ok(_) => {
                            let name = ui
                                .get_presets()
                                .iter()
                                .find(|p| p.id.as_str() == preset_id)
                                .map(|p| p.name.to_string())
                                .unwrap_or(preset_id.clone());
                            ui.set_status_text(SharedString::from(format!(
                                "Auto-apply set: {name}"
                            )));
                        }
                        Err(e) => ui.set_status_text(SharedString::from(e.to_string())),
                    }
                }
                Err(e) => ui.set_status_text(SharedString::from(e.to_string())),
            }
            refresh_audio_presets(&ui);
        }
    });

    info!("GearCraft UI (Slint) starting");
    
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
                priority: vec![dev_id.clone()],
                excluded: vec![],
                active_device_id: None,
            };
            match ipc::set_channel_device_policy(policy) {
                Ok(_) => {
                    ui.set_status_text(SharedString::from(format!(
                        "Assigned output to {ch}"
                    )));
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


    let ui_weak = ui.as_weak();
    ui.on_toggle_spatial(move || {
        if let Some(ui) = ui_weak.upgrade() {
            ui.set_spatial_enabled(!ui.get_spatial_enabled());
        }
    });

    let ui_weak = ui.as_weak();
    ui.on_toggle_smart_volume(move || {
        if let Some(ui) = ui_weak.upgrade() {
            ui.set_smart_volume(!ui.get_smart_volume());
        }
    });

    let ui_weak = ui.as_weak();
    ui.on_set_spatial_immersion(move |v| {
        if let Some(ui) = ui_weak.upgrade() {
            ui.set_spatial_immersion(v.clamp(0, 100));
        }
    });

    let ui_weak = ui.as_weak();
    ui.on_set_spatial_distance(move |v| {
        if let Some(ui) = ui_weak.upgrade() {
            ui.set_spatial_distance(v.clamp(0, 100));
        }
    });

    let ui_weak = ui.as_weak();
    ui.on_set_volume_boost(move |v| {
        if let Some(ui) = ui_weak.upgrade() {
            ui.set_volume_boost(v.clamp(-6, 12));
        }
    });

    let ui_weak = ui.as_weak();
    ui.on_apply_spatial(move || {
        if let Some(ui) = ui_weak.upgrade() {
            let fx = collect_eq_effects(&ui);
            match ipc::apply_audio_effects(fx) {
                Ok(v) => {
                    let msg = v
                        .get("message")
                        .and_then(|x| x.as_str())
                        .unwrap_or("FX applied");
                    ui.set_status_text(SharedString::from(msg));
                }
                Err(e) => ui.set_status_text(SharedString::from(e.to_string())),
            }
        }
    });


    let ui_weak = ui.as_weak();
    ui.on_toggle_eq(move || {
        if let Some(ui) = ui_weak.upgrade() {
            ui.set_eq_enabled(!ui.get_eq_enabled());
        }
    });

    let ui_weak = ui.as_weak();
    ui.on_eq_band_gain(move |index, gain| {
        if let Some(ui) = ui_weak.upgrade() {
            set_eq_gain(&ui, index, gain);
        }
    });

    let ui_weak = ui.as_weak();
    ui.on_reset_eq(move || {
        if let Some(ui) = ui_weak.upgrade() {
            ui.set_eq_bands(ModelRc::new(VecModel::from(default_eq_rows())));
            ui.set_eq_enabled(true);
            let fx = collect_eq_effects(&ui);
            match ipc::apply_audio_effects(fx) {
                Ok(_) => ui.set_status_text(SharedString::from("EQ reset to flat and applied")),
                Err(e) => ui.set_status_text(SharedString::from(format!(
                    "EQ flat (UI); apply failed: {e}"
                ))),
            }
        }
    });

    let ui_weak = ui.as_weak();
    ui.on_reset_spatial(move || {
        if let Some(ui) = ui_weak.upgrade() {
            ui.set_spatial_enabled(true);
            ui.set_spatial_immersion(40);
            ui.set_spatial_distance(40);
            ui.set_volume_boost(0);
            ui.set_smart_volume(false);
            ui.set_status_text(SharedString::from("Spatial/boost reset to defaults"));
        }
    });

    let ui_weak = ui.as_weak();
    ui.on_reset_all_fx(move || {
        if let Some(ui) = ui_weak.upgrade() {
            ui.set_eq_bands(ModelRc::new(VecModel::from(default_eq_rows())));
            ui.set_eq_enabled(true);
            ui.set_spatial_enabled(false);
            ui.set_spatial_immersion(40);
            ui.set_spatial_distance(40);
            ui.set_volume_boost(0);
            ui.set_smart_volume(false);
            let fx = collect_eq_effects(&ui);
            match ipc::apply_audio_effects(fx) {
                Ok(_) => ui.set_status_text(SharedString::from(
                    "All FX reset (flat EQ, spatial off) and applied",
                )),
                Err(e) => ui.set_status_text(SharedString::from(e.to_string())),
            }
        }
    });

    let ui_weak = ui.as_weak();
    ui.on_apply_eq(move || {
        if let Some(ui) = ui_weak.upgrade() {
            let fx = collect_eq_effects(&ui);
            match ipc::apply_audio_effects(fx) {
                Ok(v) => {
                    let msg = v
                        .get("message")
                        .and_then(|x| x.as_str())
                        .unwrap_or("EQ applied");
                    ui.set_status_text(SharedString::from(msg));
                }
                Err(e) => ui.set_status_text(SharedString::from(e.to_string())),
            }
        }
    });


    let ui_weak = ui.as_weak();
    ui.on_set_mic_gain(move |v| {
        if let Some(ui) = ui_weak.upgrade() {
            ui.set_mic_gain(v.clamp(0, 150));
        }
    });

    let ui_weak = ui.as_weak();
    ui.on_toggle_mic_mute(move || {
        if let Some(ui) = ui_weak.upgrade() {
            ui.set_mic_muted(!ui.get_mic_muted());
        }
    });

    let ui_weak = ui.as_weak();
    ui.on_toggle_mic_highpass(move || {
        if let Some(ui) = ui_weak.upgrade() {
            ui.set_mic_highpass(!ui.get_mic_highpass());
        }
    });

    let ui_weak = ui.as_weak();
    ui.on_toggle_mic_gate(move || {
        if let Some(ui) = ui_weak.upgrade() {
            ui.set_mic_noise_gate(!ui.get_mic_noise_gate());
        }
    });

    let ui_weak = ui.as_weak();
    ui.on_refresh_mic_fx(move || {
        if let Some(ui) = ui_weak.upgrade() {
            refresh_mic_fx(&ui);
        }
    });

    let ui_weak = ui.as_weak();
    ui.on_apply_mic_fx(move || {
        if let Some(ui) = ui_weak.upgrade() {
            let state = gearcraft_core::MicFxState {
                enabled: true,
                gain_pct: ui.get_mic_gain().clamp(0, 150) as u8,
                highpass: ui.get_mic_highpass(),
                highpass_hz: 100.0,
                noise_gate: ui.get_mic_noise_gate(),
                gate_strength: ui.get_mic_gate_strength().clamp(0, 100) as u8,
                lowpass: ui.get_mic_lowpass(),
                lowpass_hz: 10000.0,
                compressor: ui.get_mic_compressor(),
                compressor_strength: 45,
                rnnoise: false,
                muted: ui.get_mic_muted(),
            };
            match ipc::set_mic_fx(state) {
                Ok(v) => {
                    let msg = v
                        .get("message")
                        .and_then(|x| x.as_str())
                        .unwrap_or("Mic FX applied");
                    ui.set_status_text(SharedString::from(msg));
                }
                Err(e) => ui.set_status_text(SharedString::from(e.to_string())),
            }
        }
    });


    let ui_weak = ui.as_weak();
    ui.on_toggle_mic_lowpass(move || {
        if let Some(ui) = ui_weak.upgrade() {
            ui.set_mic_lowpass(!ui.get_mic_lowpass());
        }
    });

    let ui_weak = ui.as_weak();
    ui.on_toggle_mic_compressor(move || {
        if let Some(ui) = ui_weak.upgrade() {
            ui.set_mic_compressor(!ui.get_mic_compressor());
        }
    });

    let ui_weak = ui.as_weak();
    ui.on_open_prism(move |id| {
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
            ui.set_page(2);
            ui.set_status_text(SharedString::from("Prism — personalizza RGB"));
        }
    });

    let ui_weak = ui.as_weak();
    ui.on_set_prism_effect(move |effect| {
        if let Some(ui) = ui_weak.upgrade() {
            ui.set_prism_effect(effect);
        }
    });

    let ui_weak = ui.as_weak();
    ui.on_apply_prism(move || {
        if let Some(ui) = ui_weak.upgrade() {
            let id = ui.get_selected_device_id();
            if id.is_empty() {
                ui.set_status_text(SharedString::from("Seleziona un device in Engine"));
                return;
            }
            let zone = ui.get_color_zone().to_string();
            let effect = ui.get_prism_effect().to_string();
            let r = (ui.get_color_r() * 255.0).round().clamp(0.0, 255.0) as u8;
            let g = (ui.get_color_g() * 255.0).round().clamp(0.0, 255.0) as u8;
            let b = (ui.get_color_b() * 255.0).round().clamp(0.0, 255.0) as u8;
            let color = Some(gearcraft_core::RgbColor { r, g, b });
            match ipc::set_rgb_effect(id.as_str(), &zone, &effect, color) {
                Ok(()) => ui.set_status_text(SharedString::from(format!(
                    "Prism: {effect} su {zone} ({r},{g},{b})"
                ))),
                Err(e) => {
                    // Fallback to static color
                    match ipc::set_color(id.as_str(), &zone, r, g, b) {
                        Ok(()) => ui.set_status_text(SharedString::from(format!(
                            "Colore applicato ({r},{g},{b}) — effect API: {e}"
                        ))),
                        Err(e2) => ui.set_status_text(SharedString::from(e2.to_string())),
                    }
                }
            }
        }
    });

    // ---- Camera (under Sonar) ----
    let ui_weak = ui.as_weak();
    ui.on_refresh_cameras(move || {
        if let Some(ui) = ui_weak.upgrade() {
            refresh_cameras(&ui);
        }
    });

    let ui_weak = ui.as_weak();
    ui.on_select_camera(move |id| {
        if let Some(ui) = ui_weak.upgrade() {
            let id_s = id.to_string();
            let devices = ui.get_camera_devices();
            let mut label = SharedString::from("Nessuna camera");
            let mut path = String::new();
            for i in 0..devices.row_count() {
                if let Some(row) = devices.row_data(i) {
                    if row.id == id {
                        label = row.label.clone();
                        path = row.path.to_string();
                        break;
                    }
                }
            }
            camera::select_device(&id_s, &path);
            ui.set_camera_device_id(id.clone());
            ui.set_camera_device_label(label);
            ui.set_status_text(SharedString::from(format!("Camera: {path}")));
        }
    });

    let ui_weak = ui.as_weak();
    ui.on_toggle_camera_preview(move || {
        if let Some(ui) = ui_weak.upgrade() {
            let next = !camera::preview_active();
            let msg = camera::set_preview(next);
            ui.set_camera_preview_active(camera::preview_active());
            ui.set_camera_preview_status(SharedString::from(msg.clone()));
            ui.set_status_text(SharedString::from(msg));
        }
    });

    let ui_weak = ui.as_weak();
    ui.on_set_camera_mode(move |mode| {
        if let Some(ui) = ui_weak.upgrade() {
            ui.set_camera_mode(mode);
        }
    });

    let ui_weak = ui.as_weak();
    ui.on_set_cam_video_resolution(move |r| {
        if let Some(ui) = ui_weak.upgrade() {
            camera::update_settings(|s| s.video_resolution = r.to_string());
            ui.set_cam_video_resolution(r);
        }
    });

    let ui_weak = ui.as_weak();
    ui.on_set_cam_video_fps(move |fps| {
        if let Some(ui) = ui_weak.upgrade() {
            camera::update_settings(|s| s.video_fps = fps.max(1) as u32);
            ui.set_cam_video_fps(fps);
        }
    });

    let ui_weak = ui.as_weak();
    ui.on_set_cam_video_format(move |fmt| {
        if let Some(ui) = ui_weak.upgrade() {
            camera::update_settings(|s| s.video_format = fmt.to_string());
            ui.set_cam_video_format(fmt);
        }
    });

    let ui_weak = ui.as_weak();
    ui.on_set_cam_video_bitrate(move |br| {
        if let Some(ui) = ui_weak.upgrade() {
            let v = br.clamp(1, 50);
            camera::update_settings(|s| s.video_bitrate_mbps = v as u32);
            ui.set_cam_video_bitrate(v);
        }
    });

    let ui_weak = ui.as_weak();
    ui.on_toggle_cam_recording(move || {
        if let Some(ui) = ui_weak.upgrade() {
            let (rec, msg) = camera::toggle_recording();
            ui.set_cam_video_recording(rec);
            ui.set_camera_preview_status(SharedString::from(msg));
        }
    });

    let ui_weak = ui.as_weak();
    ui.on_set_cam_photo_resolution(move |r| {
        if let Some(ui) = ui_weak.upgrade() {
            camera::update_settings(|s| s.photo_resolution = r.to_string());
            ui.set_cam_photo_resolution(r);
        }
    });

    let ui_weak = ui.as_weak();
    ui.on_set_cam_photo_format(move |fmt| {
        if let Some(ui) = ui_weak.upgrade() {
            camera::update_settings(|s| s.photo_format = fmt.to_string());
            ui.set_cam_photo_format(fmt);
        }
    });

    let ui_weak = ui.as_weak();
    ui.on_set_cam_photo_quality(move |q| {
        if let Some(ui) = ui_weak.upgrade() {
            let v = q.clamp(10, 100) as u8;
            camera::update_settings(|s| s.photo_quality = v);
            ui.set_cam_photo_quality(v as i32);
        }
    });

    let ui_weak = ui.as_weak();
    ui.on_set_cam_photo_timer(move |sec| {
        if let Some(ui) = ui_weak.upgrade() {
            camera::update_settings(|s| s.photo_timer_s = sec.max(0) as u32);
            ui.set_cam_photo_timer(sec);
        }
    });

    let ui_weak = ui.as_weak();
    ui.on_capture_photo(move || {
        if let Some(ui) = ui_weak.upgrade() {
            let msg = camera::capture_photo();
            ui.set_status_text(SharedString::from(msg));
        }
    });

    let ui_weak = ui.as_weak();
    ui.on_set_cam_brightness(move |v| {
        if let Some(ui) = ui_weak.upgrade() {
            let v = v.clamp(0, 100) as u8;
            camera::update_settings(|s| s.brightness = v);
            ui.set_cam_brightness(v as i32);
        }
    });
    let ui_weak = ui.as_weak();
    ui.on_set_cam_contrast(move |v| {
        if let Some(ui) = ui_weak.upgrade() {
            let v = v.clamp(0, 100) as u8;
            camera::update_settings(|s| s.contrast = v);
            ui.set_cam_contrast(v as i32);
        }
    });
    let ui_weak = ui.as_weak();
    ui.on_set_cam_saturation(move |v| {
        if let Some(ui) = ui_weak.upgrade() {
            let v = v.clamp(0, 100) as u8;
            camera::update_settings(|s| s.saturation = v);
            ui.set_cam_saturation(v as i32);
        }
    });
    let ui_weak = ui.as_weak();
    ui.on_set_cam_sharpness(move |v| {
        if let Some(ui) = ui_weak.upgrade() {
            let v = v.clamp(0, 100) as u8;
            camera::update_settings(|s| s.sharpness = v);
            ui.set_cam_sharpness(v as i32);
        }
    });
    let ui_weak = ui.as_weak();
    ui.on_set_cam_exposure(move |v| {
        if let Some(ui) = ui_weak.upgrade() {
            let v = v.clamp(0, 100) as u8;
            camera::update_settings(|s| s.exposure = v);
            ui.set_cam_exposure(v as i32);
        }
    });
    let ui_weak = ui.as_weak();
    ui.on_toggle_cam_auto_exposure(move || {
        if let Some(ui) = ui_weak.upgrade() {
            camera::update_settings(|s| s.auto_exposure = !s.auto_exposure);
            ui.set_cam_auto_exposure(camera::settings().auto_exposure);
        }
    });
    let ui_weak = ui.as_weak();
    ui.on_toggle_cam_auto_wb(move || {
        if let Some(ui) = ui_weak.upgrade() {
            camera::update_settings(|s| s.auto_wb = !s.auto_wb);
            ui.set_cam_auto_wb(camera::settings().auto_wb);
        }
    });
    let ui_weak = ui.as_weak();
    ui.on_set_cam_wb_temp(move |v| {
        if let Some(ui) = ui_weak.upgrade() {
            let v = v.clamp(2500, 8000);
            camera::update_settings(|s| s.wb_temp = v as u32);
            ui.set_cam_wb_temp(v);
        }
    });

    let ui_weak = ui.as_weak();
    ui.on_toggle_camera_effect(move |id| {
        if let Some(ui) = ui_weak.upgrade() {
            camera::toggle_effect(id.as_str());
            push_effects_to_ui(&ui);
        }
    });
    let ui_weak = ui.as_weak();
    ui.on_set_camera_effect_intensity(move |id, intensity| {
        if let Some(ui) = ui_weak.upgrade() {
            camera::set_effect_intensity(id.as_str(), intensity.clamp(0, 100) as u8);
            push_effects_to_ui(&ui);
        }
    });
    let ui_weak = ui.as_weak();
    ui.on_add_camera_effect(move |kind| {
        if let Some(ui) = ui_weak.upgrade() {
            camera::add_effect(kind.as_str());
            push_effects_to_ui(&ui);
            ui.set_status_text(SharedString::from(format!("Effetto aggiunto: {kind}")));
        }
    });
    let ui_weak = ui.as_weak();
    ui.on_remove_camera_effect(move |id| {
        if let Some(ui) = ui_weak.upgrade() {
            camera::remove_effect(id.as_str());
            push_effects_to_ui(&ui);
        }
    });
    let ui_weak = ui.as_weak();
    ui.on_apply_camera_settings(move || {
        if let Some(ui) = ui_weak.upgrade() {
            ui.set_status_text(SharedString::from(camera::apply_summary()));
        }
    });



    let ui_weak = ui.as_weak();
    ui.on_take_screenshot(move |mode| {
        if let Some(ui) = ui_weak.upgrade() {
            ui.set_capture_mode(mode);
            ui.set_page(4);
            ui.set_capture_status(SharedString::from("Modalità impostata — premi Avvia cattura"));
        }
    });

    // Live camera preview → Slint Image (~30 fps)
    let ui_weak = ui.as_weak();
    let preview_timer = Timer::default();
    preview_timer.start(TimerMode::Repeated, std::time::Duration::from_millis(33), move || {
        let Some(ui) = ui_weak.upgrade() else { return };
        if !camera::preview_active() {
            return;
        }
        let Some(frame) = camera::take_frame() else { return };
        let w = frame.width;
        let h = frame.height;
        if w == 0 || h == 0 {
            return;
        }
        let need = (w as usize) * (h as usize) * 4;
        if frame.rgba.len() < need {
            return;
        }
        let mut buffer = SharedPixelBuffer::<Rgba8Pixel>::new(w, h);
        {
            let slice = buffer.make_mut_slice();
            for (dst, src) in slice.iter_mut().zip(frame.rgba.chunks_exact(4)) {
                *dst = Rgba8Pixel {
                    r: src[0],
                    g: src[1],
                    b: src[2],
                    a: src[3],
                };
            }
        }
        ui.set_camera_frame(Image::from_rgba8(buffer));
        ui.set_camera_preview_status(SharedString::from(camera::preview_status()));
    });
    // Keep timer alive for process lifetime
    std::mem::forget(preview_timer);


    let ui_weak = ui.as_weak();
    ui.on_set_capture_mode(move |mode| {
        if let Some(ui) = ui_weak.upgrade() {
            ui.set_capture_mode(mode);
        }
    });

    let ui_weak = ui.as_weak();
    ui.on_set_capture_delay(move |d| {
        if let Some(ui) = ui_weak.upgrade() {
            ui.set_capture_delay(d.clamp(0, 30));
        }
    });

    let ui_weak = ui.as_weak();
    ui.on_toggle_capture_clipboard(move || {
        if let Some(ui) = ui_weak.upgrade() {
            ui.set_capture_clipboard(!ui.get_capture_clipboard());
        }
    });

    let ui_weak = ui.as_weak();
    ui.on_start_capture(move || {
        if let Some(ui) = ui_weak.upgrade() {
            let mode = ui.get_capture_mode().to_string();
            let delay = ui.get_capture_delay().clamp(0, 30) as u8;
            let clip = ui.get_capture_clipboard();
            ui.set_capture_status(SharedString::from(if delay > 0 {
                format!("Cattura tra {delay}s…")
            } else {
                "Cattura in corso…".into()
            }));
            match ipc::take_screenshot(&mode, delay, clip) {
                Ok(v) => {
                    let msg = v
                        .get("message")
                        .and_then(|x| x.as_str())
                        .unwrap_or("OK")
                        .to_string();
                    let path = v
                        .get("path")
                        .and_then(|x| x.as_str())
                        .unwrap_or("—")
                        .to_string();
                    ui.set_capture_last_path(SharedString::from(path));
                    ui.set_capture_status(SharedString::from(msg.clone()));
                    ui.set_status_text(SharedString::from(msg));
                }
                Err(e) => {
                    ui.set_capture_status(SharedString::from(e.to_string()));
                    ui.set_status_text(SharedString::from(e.to_string()));
                }
            }
        }
    });

    // Custom window chrome (client-side decorations)
    let ui_weak = ui.as_weak();
    ui.on_win_close(move || {
        if let Some(ui) = ui_weak.upgrade() {
            let _ = ui.window().hide();
        }
        let _ = slint::quit_event_loop();
    });

    let ui_weak = ui.as_weak();
    ui.on_win_minimize(move || {
        if let Some(ui) = ui_weak.upgrade() {
            // CSD: hide as minimize fallback (backend-agnostic)
            let _ = ui.window().hide();
            ui.set_status_text(SharedString::from(
                "Minimizzata (riapri dal launcher / taskbar)",
            ));
        }
    });

    let ui_weak = ui.as_weak();
    ui.on_win_maximize(move || {
        if let Some(ui) = ui_weak.upgrade() {
            let next = !ui.get_win_maximized();
            if next {
                ui.window()
                    .set_size(slint::LogicalSize::new(1600.0, 960.0));
            } else {
                ui.window()
                    .set_size(slint::LogicalSize::new(1100.0, 720.0));
            }
            ui.set_win_maximized(next);
        }
    });

    let ui_weak = ui.as_weak();
    ui.on_open_settings(move || {
        if let Some(ui) = ui_weak.upgrade() {
            let show = !ui.get_show_settings();
            ui.set_show_settings(show);
            if show {
                ui.set_show_help(false);
            }
        }
    });

    let ui_weak = ui.as_weak();
    ui.on_open_help(move || {
        if let Some(ui) = ui_weak.upgrade() {
            let show = !ui.get_show_help();
            ui.set_show_help(show);
            if show {
                ui.set_show_settings(false);
            }
        }
    });



    let ui_weak = ui.as_weak();
    ui.on_prism_sync_all(move || {
        if let Some(ui) = ui_weak.upgrade() {
            let effect = ui.get_prism_effect().to_string();
            let zone = ui.get_color_zone().to_string();
            let r = (ui.get_color_r() * 255.0).round().clamp(0.0, 255.0) as u8;
            let g = (ui.get_color_g() * 255.0).round().clamp(0.0, 255.0) as u8;
            let b = (ui.get_color_b() * 255.0).round().clamp(0.0, 255.0) as u8;
            let color = Some(gearcraft_core::RgbColor { r, g, b });
            match ipc::prism_sync(&effect, &zone, color) {
                Ok(v) => {
                    let n = v.get("synced").and_then(|x| x.as_u64()).unwrap_or(0);
                    ui.set_status_text(SharedString::from(format!("PrismSync: {n} device(s)")));
                }
                Err(e) => ui.set_status_text(SharedString::from(e.to_string())),
            }
        }
    });

    let ui_weak = ui.as_weak();
    ui.on_set_device_dpi(move |dpi| {
        if let Some(ui) = ui_weak.upgrade() {
            let id = ui.get_selected_device_id();
            if id.is_empty() {
                ui.set_status_text(SharedString::from("Seleziona un mouse in Engine"));
                return;
            }
            match ipc::set_dpi(id.as_str(), dpi.max(50) as u32, 0) {
                Ok(v) => {
                    let msg = v.get("message").and_then(|x| x.as_str()).unwrap_or("DPI set");
                    ui.set_mouse_dpi(dpi);
                    ui.set_mouse_dpi_label(SharedString::from(format!("DPI: {dpi}")));
                    ui.set_status_text(SharedString::from(msg));
                }
                Err(e) => ui.set_status_text(SharedString::from(e.to_string())),
            }
        }
    });

    let ui_weak = ui.as_weak();
    ui.on_refresh_mouse_info(move || {
        if let Some(ui) = ui_weak.upgrade() {
            let id = ui.get_selected_device_id();
            if id.is_empty() {
                ui.set_mouse_dpi_label(SharedString::from("DPI: —"));
                return;
            }
            match ipc::get_mouse_info(id.as_str()) {
                Ok(v) => {
                    if let Some(d) = v.get("dpi").and_then(|x| x.as_u64()) {
                        ui.set_mouse_dpi(d as i32);
                        ui.set_mouse_dpi_label(SharedString::from(format!("DPI: {d}")));
                    } else {
                        ui.set_mouse_dpi_label(SharedString::from("DPI: n/d"));
                    }
                }
                Err(e) => ui.set_mouse_dpi_label(SharedString::from(e.to_string())),
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
    let balance = v
        .get("balance")
        .and_then(|x| x.as_i64())
        .unwrap_or(0) as i32;
    let game = v
        .get("game_volume")
        .and_then(|x| x.as_u64().or_else(|| x.as_f64().map(|f| f as u64)))
        .unwrap_or(100) as i32;
    let chat = v
        .get("chat_volume")
        .and_then(|x| x.as_u64().or_else(|| x.as_f64().map(|f| f as u64)))
        .unwrap_or(100) as i32;
    let backend = v
        .get("backend")
        .and_then(|x| x.as_str())
        .unwrap_or("—");
    let ready = v
        .get("available")
        .and_then(|x| x.as_bool())
        .unwrap_or(false);

    ui.set_chatmix_balance(balance);
    ui.set_game_volume(game.clamp(0, 100));
    ui.set_chat_volume(chat.clamp(0, 100));
    ui.set_chatmix_backend(SharedString::from(backend));
    ui.set_chatmix_ready(ready);

    update_channel(ui, "game", |ch| ch.volume = game.clamp(0, 100));
    update_channel(ui, "chat", |ch| ch.volume = chat.clamp(0, 100));
}

fn update_channel(ui: &App, id: &str, mut f: impl FnMut(&mut ChannelRow)) {
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

/// Preset id bound to `device_id` by an auto-apply rule (exact route wins,
/// then the route-less rule). Err when the daemon can't be reached.
fn auto_rule_for(device_id: &str) -> Result<Option<String>, ipc::IpcError> {
    let rules = ipc::list_preset_rules()?;
    let Value::Array(rules) = rules else {
        return Ok(None);
    };
    let mut route_less: Option<String> = None;
    for r in &rules {
        let d = r.get("device_id").and_then(|v| v.as_str()).unwrap_or("");
        if d != device_id {
            continue;
        }
        let preset = r
            .get("preset_id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let route = r.get("route_id").and_then(|v| v.as_str()).unwrap_or("");
        if route.is_empty() {
            return Ok(Some(preset));
        }
        if route_less.is_none() {
            route_less = Some(preset);
        }
    }
    Ok(route_less)
}

fn refresh_audio_presets(ui: &App) {
    let selected = ui.get_selected_device_id().to_string();
    let auto_id = if selected.is_empty() {
        None
    } else {
        auto_rule_for(&selected).unwrap_or(None)
    };

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
                let is_auto = auto_id.as_deref() == Some(id.as_str());
                rows.push(PresetRow {
                    id: SharedString::from(id),
                    name: SharedString::from(name),
                    notes: SharedString::from(notes),
                    eq_enabled: eq,
                    auto: is_auto,
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
                    // Prefer physical in list order; still show virtual dimmed
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
                        // shorten long sink names
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
            // soft: daemon may be old
            ui.set_status_text(SharedString::from(format!("Device Manager: {e}")));
        }
    }
}

fn refresh_mic_fx(ui: &App) {
    match ipc::get_mic_fx() {
        Ok(v) => {
            if let Some(g) = v.get("gain_pct").and_then(|x| x.as_u64()) {
                ui.set_mic_gain(g.min(150) as i32);
            }
            if let Some(m) = v.get("muted").and_then(|x| x.as_bool()) {
                ui.set_mic_muted(m);
            }
            if let Some(h) = v.get("highpass").and_then(|x| x.as_bool()) {
                ui.set_mic_highpass(h);
            }
            if let Some(g) = v.get("noise_gate").and_then(|x| x.as_bool()) {
                ui.set_mic_noise_gate(g);
            }
            if let Some(s) = v.get("gate_strength").and_then(|x| x.as_u64()) {
                ui.set_mic_gate_strength(s.min(100) as i32);
            }
        }
        Err(_) => {}
    }
}


fn refresh_cameras(ui: &App) {
    let devices = camera::list_devices();
    let mut rows = Vec::new();
    for d in &devices {
        rows.push(CameraDeviceRow {
            id: SharedString::from(d.id.as_str()),
            label: SharedString::from(d.label.as_str()),
            path: SharedString::from(d.path.as_str()),
            available: d.available,
        });
    }
    if ui.get_camera_device_id().is_empty() {
        if let Some(first) = devices.first() {
            camera::select_device(&first.id, &first.path);
            ui.set_camera_device_id(SharedString::from(first.id.as_str()));
            ui.set_camera_device_label(SharedString::from(first.label.as_str()));
        }
    }
    ui.set_camera_devices(ModelRc::new(VecModel::from(rows)));
    ui.set_camera_preview_active(camera::preview_active());
    ui.set_cam_video_recording(camera::recording());
    let s = camera::settings();
    ui.set_cam_video_resolution(SharedString::from(s.video_resolution.as_str()));
    ui.set_cam_video_fps(s.video_fps as i32);
    ui.set_cam_video_format(SharedString::from(s.video_format.as_str()));
    ui.set_cam_video_bitrate(s.video_bitrate_mbps as i32);
    ui.set_cam_photo_resolution(SharedString::from(s.photo_resolution.as_str()));
    ui.set_cam_photo_format(SharedString::from(s.photo_format.as_str()));
    ui.set_cam_photo_quality(s.photo_quality as i32);
    ui.set_cam_photo_timer(s.photo_timer_s as i32);
    ui.set_cam_brightness(s.brightness as i32);
    ui.set_cam_contrast(s.contrast as i32);
    ui.set_cam_saturation(s.saturation as i32);
    ui.set_cam_sharpness(s.sharpness as i32);
    ui.set_cam_exposure(s.exposure as i32);
    ui.set_cam_auto_exposure(s.auto_exposure);
    ui.set_cam_auto_wb(s.auto_wb);
    ui.set_cam_wb_temp(s.wb_temp as i32);
    push_effects_to_ui(ui);
}

fn push_effects_to_ui(ui: &App) {
    let s = camera::settings();
    let rows: Vec<CameraEffectRow> = s
        .effects
        .iter()
        .map(|e| CameraEffectRow {
            id: SharedString::from(e.id.as_str()),
            name: SharedString::from(e.name.as_str()),
            kind: SharedString::from(e.kind.as_str()),
            enabled: e.enabled,
            intensity: e.intensity as i32,
        })
        .collect();
    ui.set_camera_effects(ModelRc::new(VecModel::from(rows)));
}

