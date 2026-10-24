# GearCraft Roadmap

Open-source, brand-agnostic peripheral stack for Linux (RGB, macros, ChatMix, profiles).

**License**: GPL-3.0-or-later

## Phase 1 — Core + SteelSeries 🔄

- [x] Cargo workspace (core / daemon / ui)
- [x] HID scan + SteelSeries addons (Arctis, Apex, Rival 3)
- [x] Unix socket IPC
- [x] Slint UI (Home / Engine / Sonar)
- [x] Packaging (udev, systemd user unit, install.sh)

## Phase 2 — Profiles & packaging 🔄

- [x] Macro store + uinput playback
- [x] ChatMix (PipeWire null sinks)
- [ ] AUR package
- [ ] Flatpak (optional)

## Phase 3 — Input & audio depth 🔄

- [x] **Audio presets** (JDSP4Linux-style CRUD + device rules + Sonar UI) — see `docs/AUDIO_PRESETS.md`
- [x] WirePlumber filter-chain DSP bridge (EQ, crossfeed) — `dsp_pw`
- [ ] Auto-apply preset rules on headset connect
- [ ] AutoEQ import
- [ ] Convolver IR attach

## Phase 4 — Multi-brand

- [ ] Razer addon skeleton
- [ ] Logitech (open protocol subset)
- [ ] Plugin discovery path
