# Push status (updated 2026-10-30)

## UI source of truth

`crates/gearcraft-ui/ui/app.slint` in this repository **is the full UI again**
(2 747 lines). It was replaced by a `PLACEHOLDER`/restore-stub in the
2026-10-27 session (the `app.slint.gz.b64.part*` blobs were never committed),
so the restored file was taken from the last commit that still had it:
`1b2b9a8`.

`scripts/restore-app-slint.sh` and `ui/README_SLINT.md` are kept for reference
but are no longer needed — the `.part*` files do not exist in this tree.

## Wiring status (2026-10-30)

- All Slint callbacks declared in `app.slint` are wired in
  `crates/gearcraft-ui/src/main.rs` (camera, Prism/DPI, screen capture,
  FX page, Mic FX, window chrome).
- `crates/gearcraft-ui/src/camera.rs` is compiled again (`mod camera;`).
- The daemon compiles `drivers/`, `mic_fx.rs`, `screenshot.rs`,
  `game_presets.rs` and `jamesdsp_import.rs` again, and exposes them over IPC
  (`GetMouseInfo`, `SetDpi`, `SetEffect`, `PrismSync`, `GetMicFx`, `SetMicFx`,
  `ApplyAudioEffects`, `CaptureScreenshot`, `ImportJamesDspPreset`,
  `ImportJamesDspDir`) — see [PROTOCOL.md](../PROTOCOL.md).
- `MicFxState` now lives in `gearcraft-core` (it was referenced by `mic_fx.rs`
  but missing, which is why that module was dropped).

## Windows packaging

- `packaging/windows/*`, `scripts/cross-windows.sh` are present.
- `audio_windows.rs` is still a WASAPI/APO placeholder.
