# GearCraft

Open-source Linux alternative to SteelSeries GG. Multi-brand peripherals via a plugin/addon system. Written in Rust.

**License**: [GPL-3.0-or-later](LICENSE)  
**UI**: Tauri 2 + Svelte 5 + Tailwind  
**Daemon**: HID + Unix socket IPC + PipeWire ChatMix  
**Design**: dark theme, accent `#00E7A0`

## Project layout

```
gearcraft/
├── Cargo.toml                 # workspace
├── PROTOCOL.md                # IPC protocol
├── ROADMAP.md
├── crates/
│   ├── gearcraft-core/
│   ├── gearcraft-daemon/
│   └── gearcraft-ui/
├── assets/udev/
├── docs/
└── packaging/
```

## Quick start

```bash
# Daemon
cargo run -p gearcraft-daemon

# UI
cd crates/gearcraft-ui && npm install && npm run tauri dev
```

Live USB: `cargo run -p gearcraft-daemon --features hidapi` (needs `libhidapi`).

Macros need write access to `/dev/uinput` (group `input` or root).

See [ROADMAP.md](ROADMAP.md), [docs/CHATMIX.md](docs/CHATMIX.md), [docs/STEELSERIES.md](docs/STEELSERIES.md).
