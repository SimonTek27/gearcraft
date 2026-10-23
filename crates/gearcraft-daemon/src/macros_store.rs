//! In-memory macro store (persist later to disk).

use std::collections::HashMap;

use gearcraft_core::MacroDef;
use tracing::info;

use crate::uinput_macros;

#[derive(Default)]
pub struct MacroStore {
    macros: HashMap<String, MacroDef>,
}

impl MacroStore {
    pub fn new_with_defaults() -> Self {
        let mut s = Self::default();
        s.upsert(MacroDef {
            id: "demo-clip".into(),
            name: "Clip highlight".into(),
            device_id: None,
            trigger: "Mouse button 4".into(),
            actions: vec![gearcraft_core::MacroAction {
                key: "F8".into(),
                delay_ms: 0,
            }],
            enabled: true,
        });
        s.upsert(MacroDef {
            id: "demo-mute".into(),
            name: "Mute mic".into(),
            device_id: None,
            trigger: "Mouse button 5".into(),
            actions: vec![gearcraft_core::MacroAction {
                key: "ctrl+shift+m".into(),
                delay_ms: 50,
            }],
            enabled: true,
        });
        s
    }

    pub fn list(&self) -> Vec<MacroDef> {
        let mut v: Vec<_> = self.macros.values().cloned().collect();
        v.sort_by(|a, b| a.name.cmp(&b.name));
        v
    }

    pub fn upsert(&mut self, m: MacroDef) {
        self.macros.insert(m.id.clone(), m);
    }

    pub fn delete(&mut self, id: &str) -> bool {
        self.macros.remove(id).is_some()
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
}
