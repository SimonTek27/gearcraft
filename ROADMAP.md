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
- [x] Live USB scanning via `hidapi` (on by default, demo fallback)
- [x] udev rules (`assets/udev/99-gearcraft.rules`)
- [x] Packaging: systemd unit, `install.sh`, Debian skeleton, AUR snippet

## Phase 2 — UX features ✅

- [x] Home → live device list from daemon
- [x] Engine → colour picker + zone apply (`set_color`)
- [x] Macros store + Engine UI (add / test / delete) — execution stub
- [x] Sonar → headset battery display
- [x] ChatMix software + **PipeWire/Pulse** volume apply (`docs/CHATMIX.md`)

## Phase 3 — Input & audio depth 🔄

- [x] **Audio presets** (JDSP4Linux-style CRUD + device rules + Sonar UI) — see `docs/AUDIO_PRESETS.md`
- [x] PipeWire filter-chain DSP bridge (EQ, crossfeed) — `dsp_pw`
- [x] Auto-apply preset rules on headset connect
- [x] Game preset auto-switch (Quick Set style)

- [x] **uinput** real macro playback + optional key grab
- [x] Persist macros + ChatMix to disk (`~/.config/gearcraft/`)
- [ ] WirePlumber static routes / session profiles for ChatMix
- [ ] Hardware ChatMix dial read when available

## Phase 4 — Hardware completeness

- [x] Hardware test harness (`docs/HARDWARE.md`, `packaging/hw-test.sh`, `GEARCRAFT_SCAN_ONLY`)
- [ ] Live validation matrix filled on real mice / keyboards / headsets
- [ ] Per-key / multi-zone keyboard RGB from open captures
- [x] Live battery HID read path (Arctis write+read; validate on hardware)
- [x] Hotplug via udev monitor (replace periodic rescan)

## Phase 5 — More brands (plugin model)

- [x] Razer addon (OpenRazer-compat RGB: logo/scroll/matrix)
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

1. Hardware validation on real devices (`--features hidapi`)  
2. First AUR publish  
3. WirePlumber static ChatMix routes  
4. Per-device colour profile persistence  
5. Razer HID RGB write (open captures)  

Contributions welcome — open an issue for device capture logs (Wireshark/USBPcap) or PipeWire routing tips.
