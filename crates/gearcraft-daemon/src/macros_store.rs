//! Macro store with XDG disk persistence (`~/.config/gearcraft/macros.json`).

use std::collections::HashMap;

use gearcraft_core::MacroDef;
use tracing::{info, warn};

use crate::config;
use crate::uinput_macros;

#[derive(Default)]
pub struct MacroStore {
    macros: HashMap<String, MacroDef>,
}

impl MacroStore {
    /// Load from disk; if missing or empty, seed demo macros and persist them.
    pub fn load_or_defaults() -> Self {
        match config::load_macros() {
            Some(list) if !list.is_empty() => {
                let mut s = Self::default();
                for m in list {
                    s.macros.insert(m.id.clone(), m);
                }
                s
            }
            _ => {
                let s = Self::new_with_defaults();
                if let Err(e) = s.persist() {
                    warn!("failed to write default macros: {e}");
                }
                s
            }
        }
    }

    pub fn new_with_defaults() -> Self {
        let mut s = Self::default();
        s.macros.insert(
            "demo-clip".into(),
            MacroDef {
                id: "demo-clip".into(),
                name: "Clip highlight".into(),
                device_id: None,
                trigger: "Mouse button 4".into(),
                actions: vec![gearcraft_core::MacroAction {
                    key: "F8".into(),
                    delay_ms: 0,
                }],
                enabled: true,
            },
        );
        s.macros.insert(
            "demo-mute".into(),
            MacroDef {
                id: "demo-mute".into(),
                name: "Mute mic".into(),
                device_id: None,
                trigger: "Mouse button 5".into(),
                actions: vec![gearcraft_core::MacroAction {
                    key: "ctrl+shift+m".into(),
                    delay_ms: 50,
                }],
                enabled: true,
            },
        );
        s
    }

    pub fn list(&self) -> Vec<MacroDef> {
        let mut v: Vec<_> = self.macros.values().cloned().collect();
        v.sort_by(|a, b| a.name.cmp(&b.name));
        v
    }

    pub fn upsert(&mut self, m: MacroDef) {
        self.macros.insert(m.id.clone(), m);
        if let Err(e) = self.persist() {
            warn!("macro persist failed: {e}");
        }
    }

    pub fn delete(&mut self, id: &str) -> bool {
        let removed = self.macros.remove(id).is_some();
        if removed {
            if let Err(e) = self.persist() {
                warn!("macro persist failed: {e}");
            }
        }
        removed
    }

    pub fn run(&self, id: &str) -> Result<bool, String> {
        let m = self
            .macros
            .get(id)
            .ok_or_else(|| format!("macro not found: {id}"))?;
        if !m.enabled {
            return Err("macro disabled".into());
        }
        info!("running macro '{}'", m.name);
        uinput_macros::play_macro(m)
    }

    fn persist(&self) -> Result<(), String> {
        let list = self.list();
        config::save_macros(&list)?;
        info!("macros saved ({} entries)", list.len());
        Ok(())
    }
}
