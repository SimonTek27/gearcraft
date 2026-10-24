//! GearCraft daemon — HID + IPC + ChatMix + audio presets + WirePlumber DSP.

mod addons;
mod audio_presets;
mod chatmix_pw;
mod dsp_pw;
mod config;
mod ipc;
mod macros_store;
mod uinput_macros;
mod usb;

use std::sync::Arc;

use audio_presets::AudioPresetStore;
use macros_store::MacroStore;
use tokio::sync::RwLock;
use tracing::info;

use crate::ipc::SharedState;
use crate::usb::DeviceRegistry;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    info!("GearCraft daemon starting");

    let settings = config::load_settings();
    let mut registry = DeviceRegistry::new();
    registry.scan();

    let macros = MacroStore::load_or_defaults();
    info!("macros in store: {}", macros.list().len());

    let audio_presets = AudioPresetStore::load();

    let state = SharedState {
        registry: Arc::new(RwLock::new(registry)),
        macros: Arc::new(RwLock::new(macros)),
        chatmix: Arc::new(RwLock::new(settings.chatmix)),
        audio_presets: Arc::new(RwLock::new(audio_presets)),
    };

    ipc::serve(state).await?;
    Ok(())
}
