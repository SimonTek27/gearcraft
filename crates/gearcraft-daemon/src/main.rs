//! GearCraft daemon – background service for USB peripherals and IPC.

mod addons;
mod app_routing;
mod hotplug;
mod device_manager;
mod audio_presets;
mod chatmix_pw;
mod dsp_pw;
mod config;
mod ipc;
mod macros_store;
mod uinput_macros;
mod usb;

use std::sync::Arc;

use tokio::sync::RwLock;
use tracing::{info, Level};
use tracing_subscriber::EnvFilter;

use ipc::{run_server, DeviceRegistry, SharedState};
use audio_presets::AudioPresetStore;
use macros_store::MacroStore;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_max_level(Level::DEBUG)
        .init();

    info!("GearCraft daemon starting…");
    info!("config dir: {}", config::config_dir().display());

    if usb::is_scan_only() {
        usb::print_scan_report();
        return Ok(());
    }

    if usb::is_dry_run() {
        info!("Dry-run mode (GEARCRAFT_DRY=1)");
    }

    let settings = config::load_settings();
    let backend = chatmix_pw::detect_sinks().await;
    info!("ChatMix audio: {} – {}", backend.backend, backend.message);

    if backend.available {
        let g = settings.chatmix.game_volume;
        let c = settings.chatmix.chat_volume;
        match chatmix_pw::apply_volumes(g, c).await {
            Ok(_) => info!("restored ChatMix volumes from config"),
            Err(e) => info!("ChatMix restore skipped: {e}"),
        }
    }

    let mut registry = DeviceRegistry::new();
    registry.refresh();

    let macros = MacroStore::load_or_defaults();
    let audio_presets = AudioPresetStore::load();
    let device_manager = device_manager::DeviceManager::load();

    let state = SharedState {
        registry: Arc::new(RwLock::new(registry)),
        macros: Arc::new(RwLock::new(macros)),
        chatmix: Arc::new(RwLock::new(settings.chatmix)),
        audio_presets: Arc::new(RwLock::new(audio_presets)),
        device_manager: Arc::new(RwLock::new(device_manager)),
    };

    tokio::spawn(async move {
        app_routing::run_loop(2).await;
    });

    let reg_hp = Arc::clone(&state.registry);
    let presets_hp = Arc::clone(&state.audio_presets);
    tokio::spawn(async move {
        hotplug::run(reg_hp, presets_hp).await;
    });

    let dm_tick = Arc::clone(&state.device_manager);
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(3));
        loop {
            interval.tick().await;
            let mut dm = dm_tick.write().await;
            device_manager::failover_tick(&mut dm).await;
        }
    });

    {
        let mut dm = state.device_manager.write().await;
        if dm.state.auto_switch_on_startup {
            match device_manager::apply(&mut dm).await {
                Ok(v) => info!("device manager startup apply: {v}"),
                Err(e) => info!("device manager startup apply skipped: {e}"),
            }
        }
    }

    run_server(state).await?;
    Ok(())
}
