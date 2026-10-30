# GearCraft

**Open-source peripheral control for Linux** — RGB, macros, headset mix and device profiles, without locking you to a single vendor.

GearCraft is a brand-agnostic desktop stack: a background daemon talks to USB/HID devices through plugins, and a modern UI lets you manage them. New brands are added as addons; software features (ChatMix, macros, profiles) work across devices.

**License**: [GPL-3.0-or-later](LICENSE)  
**UI**: Slint (Rust native)  
**Daemon**: hidapi + Unix socket IPC  
**Design**: dark theme, accent `#00E7A0`

## Why

Vendor control apps on Linux are scarce or incomplete. GearCraft aims to be one place for:

- Lighting and simple profiles on supported mice / keyboards / headsets  
- Software ChatMix (game vs chat balance) on PipeWire  
- Macro playback via uinput  
- A clean plugin model so the project is **not** tied to one brand

Supported and planned devices are listed by category in [docs/devices.md](docs/devices.md).

## Project layout

```
gearcraft/
├── Cargo.toml                 # workspace
├── PROTOCOL.md                # IPC protocol
├── crates/
│   ├── gearcraft-core/        # types + PeripheralAddon trait + IPC messages
│   ├── gearcraft-daemon/      # background service + brand addons
│   └── gearcraft-ui/          # Slint desktop UI
├── docs/
│   ├── devices.md             # device matrix by category
│   └── CHATMIX.md
└── packaging/                 # systemd, deb, install scripts
```

## Quick start (development)

### 1. Daemon

```bash
cd gearcraft
cargo run -p gearcraft-daemon
```

Listens on `$XDG_RUNTIME_DIR/gearcraft.sock` (or `/tmp/gearcraft.sock`).

### 2. UI

```bash
cd crates/gearcraft-ui
cargo run -p gearcraft-ui
```

Requires a working Slint stack (Rust, system libs for the Slint backend — e.g. libxcb on X11).

## Status

- [x] Workspace + core types + IPC protocol  
- [x] Daemon Unix-socket server (demo devices + first HID addon)  
- [x] UI shell (custom titlebar, sidebar, Home / Engine / Sonar)  
- [x] PipeWire ChatMix + uinput macros  
- [x] Config persistence (`~/.config/gearcraft/`)  
- [ ] Broader brand coverage (see [devices.md](docs/devices.md))  
- [ ] Packaging polish (AUR, wider udev coverage)

## Install daemon (systemd)

```bash
# install daemon (live USB built in) + udev rules
sudo ./packaging/install.sh

# demo-only build (no HID, for machines without peripherals)
cargo build -p gearcraft-daemon --no-default-features

sudo systemctl enable --now gearcraft-daemon
```

## Build .deb

```bash
./packaging/build-deb.sh
# output: packaging/gearcraft-daemon_0.1.0-1_amd64.deb
sudo dpkg -i packaging/gearcraft-daemon_*.deb
```

## ChatMix (PipeWire)

See [docs/CHATMIX.md](docs/CHATMIX.md). In the UI: **Sonar → Setup Game/Chat sinks**, then move the balance slider.

## Contributing devices

1. Pick a category in [docs/devices.md](docs/devices.md).  
2. Implement `PeripheralAddon` for that brand/family.  
3. Extend udev rules and document VID/PID in the matrix.  

No brand is privileged in the architecture — first-party and community addons share the same trait.
