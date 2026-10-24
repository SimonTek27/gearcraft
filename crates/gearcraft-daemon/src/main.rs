//! GearCraft daemon – background service for USB peripherals and IPC.

mod addons;
mod app_routing;
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
    info!(
        "ChatMix balance from config: {} (game {}% / chat {}%)",
        settings.chatmix.balance, settings.chatmix.game_volume, settings.chatmix.chat_volume
    );

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
    info!("macros in store: {}", macros.list().len());

    let audio_presets = AudioPresetStore::load();

    let state = SharedState {
        registry: Arc::new(RwLock::new(registry)),
        macros: Arc::new(RwLock::new(macros)),
        chatmix: Arc::new(RwLock::new(settings.chatmix)),
        audio_presets: Arc::new(RwLock::new(audio_presets)),
    };

    let reg_scan = Arc::clone(&state.registry);
    // Auto-route apps → Sonar channels (Discord→chat, Haruna→media, …)
    tokio::spawn(async move {
        app_routing::run_loop(2).await;
    });

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(15));
        loop {
            interval.tick().await;
            let mut reg = reg_scan.write().await;
            reg.refresh();
        }
    });

    run_server(state).await?;
    Ok(())
}
