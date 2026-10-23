//! GearCraft daemon – background service for USB peripherals and IPC.

mod addons;
mod chatmix_pw;
mod ipc;
mod macros_store;
mod uinput_macros;
mod usb;

use std::sync::Arc;

use tokio::sync::RwLock;
use tracing::{info, Level};
use tracing_subscriber::EnvFilter;

use gearcraft_core::ChatMixState;
use ipc::{run_server, DeviceRegistry, SharedState};
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
    if usb::is_dry_run() {
        info!("Dry-run mode (GEARCRAFT_DRY=1)");
    }

    let backend = chatmix_pw::detect_sinks().await;
    info!("ChatMix audio: {} – {}", backend.backend, backend.message);

    let mut registry = DeviceRegistry::new();
    registry.refresh();

    let state = SharedState {
        registry: Arc::new(RwLock::new(registry)),
        macros: Arc::new(RwLock::new(MacroStore::new_with_defaults())),
        chatmix: Arc::new(RwLock::new(ChatMixState::default())),
    };

    let reg_scan = Arc::clone(&state.registry);
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
