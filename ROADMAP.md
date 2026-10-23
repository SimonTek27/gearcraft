# GearCraft Roadmap

Open-source, brand-agnostic peripheral stack for Linux (RGB, macros, ChatMix, profiles).

**License**: GPL-3.0-or-later  
**Stack**: Rust workspace · Tauri 2 · Svelte 5 · PipeWire · HID

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

- [x] Home → live device list from daemon
- [x] Engine → colour picker + zone apply (`set_color`)
- [x] Macros store + Engine UI (add / test / delete) — execution stub
- [x] Sonar → headset battery display
- [x] ChatMix software + **PipeWire/Pulse** volume apply (`docs/CHATMIX.md`)

## Phase 3 — Input & audio depth 🔄

- [x] **uinput** real macro playback + optional key grab
- [ ] Persist macros to disk (`~/.config/gearcraft/macros.json`)
- [ ] WirePlumber static routes / session profiles for ChatMix
- [ ] Hardware ChatMix dial read when available

## Phase 4 — Hardware completeness

- [ ] Live testing matrix (mice / keyboards / headsets) with `--features hidapi`
- [ ] Per-key / multi-zone keyboard RGB from open captures
- [ ] Live battery HID read + charge state
- [ ] Hotplug via udev monitor (replace periodic rescan)

## Phase 5 — More brands (plugin model)

- [ ] Razer addon skeleton
- [ ] Logitech (Lightspeed / G HUB subset)
- [ ] Corsair / iCUE-compatible subset
- [ ] Addon discovery API for third-party plugins

## Phase 6 — Distribution

- [ ] AUR package (`gearcraft-daemon`, `gearcraft-ui`)
- [ ] Flathub / AppImage for UI
- [ ] COPR / OBS builds
- [ ] Signed releases + changelog automation

## Phase 7 — Polish

- [ ] Profiles (per-game) and auto-switch
- [ ] Onboarding wizard (udev, sinks, permissions)
- [ ] Accessibility & localization
- [ ] CI: `cargo test`, UI typecheck, packaging dry-run

---

## Near-term priority (next 2–4 weeks)

1. uinput macro execution  
2. Config persistence  
3. Hardware validation on real devices  
4. First AUR publish  

Contributions welcome — open an issue for device capture logs (Wireshark/USBPcap) or PipeWire routing tips.
