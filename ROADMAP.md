# GearCraft Roadmap

Open-source, brand-agnostic peripheral stack for Linux (RGB, macros, ChatMix, profiles).

**License**: GPL-3.0-or-later  
**Stack**: Rust workspace · Slint · PipeWire · HID

---

## Phase 0 — Foundation ✅

- [x] Project name, architecture, design system (dark UI, accent `#00E7A0`)
- [x] Cargo workspace: `gearcraft-core`, `gearcraft-daemon`, `gearcraft-ui`
- [x] IPC protocol (Unix socket, length-prefixed JSON) — see [PROTOCOL.md](PROTOCOL.md)
- [x] UI shell: custom titlebar, sidebar, Home / Engine / Sonar
- [x] GPL-3 license

## Phase 1 — Daemon & devices ✅

- [x] Device registry + demo devices
- [x] First HID addon family (mice / keyboards / headsets):
  - [x] Rival 3 family (RGB zones, polling, effects)
  - [x] Apex keyboards (uniform RGB)
  - [x] Arctis headsets (battery query stubs)
- [x] Optional `hidapi` feature for live USB
- [x] udev rules (`assets/udev/99-gearcraft.rules`)
- [x] Packaging: systemd unit, `install.sh`, Debian skeleton, AUR snippet

## Phase 2 — UX features ✅

- [x] Software ChatMix (PipeWire null sinks + balance)
- [x] Macro store + uinput playback
- [x] Config persistence under XDG

## Phase 3 — Harden & ship (next)

- [ ] Live hardware validation on user machines
- [ ] AUR package
- [ ] Broader device matrix
- [ ] Profile export/import
