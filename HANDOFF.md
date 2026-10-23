# GearCraft - Documento di Handoff
## 📋 Contesto del Progetto
**Nome**: GearCraft  
**Obiettivo**: Creare un'alternativa open source a SteelSeries GG per Linux, scritta in Rust, con supporto multi-marca (non solo SteelSeries) tramite sistema a plugin/addon.  
**Ispirazione UI**: SteelSeries GG (design scuro, moderno, card grandi, verde acqua brillante)  
**Data**: 23 Ottobre 2026
**Repo**: https://github.com/SimonTek27/gearcraft
---
## 🛠 Stack Tecnologico
### Backend (Rust)
- **Linguaggio**: Rust
- **Struttura**: Cargo Workspace con 3 crate
  - `gearcraft-core`: Trait comuni, tipi di dati, protocolli
  - `gearcraft-daemon`: Servizio background, comunicazione USB (hidapi), IPC
  - `gearcraft-ui`: Interfaccia grafica (Tauri v2)
- **Comunicazione USB**: `hidapi` (wrapper per hidraw)
- **IPC (UI ↔ Daemon)**: Unix Socket con tokio + serde (JSON)
- **Gestione input/macro**: `evdev-rs` + `uinput`
- **Permessi Linux**: File udev rules per accesso a `/dev/hidraw*`
### Frontend (UI)
- **Framework**: Tauri v2 + Svelte 5 + TypeScript
- **Styling**: TailwindCSS
- **Design System**: Palette SteelSeries GG (nero profondo + verde acqua #00E7A0)
---
## 🎨 Design dell'Interfaccia
### Layout
```
┌──────────────────────────────────────────────────────────────────────────────┐
│  ● GearCraft                              [⚙] [?] [─] [□] [✕]             │
├──────────────┬───────────────────────────────────────────────────────────────┤
│              │                                                               │
│  🏠 Home     │                                                               │
│  🎮 Engine   │                     (contenuto principale)                    │
│  🔊 Sonar    │                                                               │
│  ⚙️ Settings │                                                               │
│              │                                                               │
└──────────────┴───────────────────────────────────────────────────────────────┘
```
Custom titlebar (no native chrome), dark theme, accent `#00E7A0`.

---
## 📁 Struttura del Repository
```
gearcraft/
├── Cargo.toml                 # workspace
├── crates/
│   ├── gearcraft-core/        # tipi + PeripheralAddon trait
│   ├── gearcraft-daemon/      # daemon + addons + IPC + uinput + ChatMix
│   └── gearcraft-ui/          # Tauri + Svelte 5
├── assets/udev/
├── packaging/                 # deb, systemd, install.sh, AUR snippet
├── docs/
├── HANDOFF.md
├── PROTOCOL.md
├── ROADMAP.md
└── LICENSE                    # GPL-3.0
```

---
## ✅ Stato attuale (2026-10-23)
- Workspace Cargo completo
- Daemon IPC (Unix socket, length-prefixed JSON)
- SteelSeries addon (Rival 3, Apex, Arctis) con demo devices
- PipeWire ChatMix (wpctl/pactl)
- Macro store + uinput playback
- UI shell completa (Home / Engine / Sonar) collegata al daemon
- Packaging deb + systemd + udev
- Repo pubblico: https://github.com/SimonTek27/gearcraft

### Prossimi passi prioritari
1. **Push completo del tree** (questo commit)
2. Test hardware reale con `--features hidapi`
3. Persistenza configurazione (RGB, DPI, macros)
4. Per-key RGB Apex + lettura batteria live
5. Hotkey grab per macro
6. Pacchetto AUR + deb finale

---
## 🔌 Protocollo IPC (riassunto)
Socket: `/run/user/$UID/gearcraft.sock` (o `$XDG_RUNTIME_DIR`)
Messaggi: `u32 LE length` + JSON UTF-8.
Comandi principali: `ListDevices`, `GetDevice`, `SetColor`, `SetDpi`, `ListMacros`, `SaveMacro`, `PlayMacro`, `DeleteMacro`, `GetChatMix`, `SetChatMix`, `SetupChatMix`.
Vedi `PROTOCOL.md` per schema completo.

---
## 🏗 Come buildare
```bash
# Daemon (senza hardware)
cargo build -p gearcraft-daemon
# Con HID reale
cargo build -p gearcraft-daemon --features hidapi

# UI
cd crates/gearcraft-ui && npm install && npm run tauri dev
```

---
## 📝 Note per il successore
- Nessun device HID reale nel sandbox → demo devices quando hidapi off
- `/dev/uinput` richiede udev + gruppo; fallback log-only se permission denied
- Palette e layout ispirati a SteelSeries GG ma completamente originali
- Multi-brand: aggiungere nuovo addon implementando `PeripheralAddon` in `gearcraft-core`
