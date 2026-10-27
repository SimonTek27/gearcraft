# Push status (2026-10-27)

## On GitHub `main`
- Docs: `CAMERA_UI.md`, `UI_PALETTE.md`, `STREAMER_CHATMIX.md`, `WINDOWS.md`
- Windows packaging: `packaging/windows/*`, `scripts/cross-windows.sh`
- `audio_windows.rs` stub, Cargo.toml v4l linux-only
- `scripts/restore-app-slint.sh` (expects blob parts when present)
- `crates/gearcraft-ui/ui/app.slint` currently a **short stub** (restore message) because the full ~193KB file exceeds the push channel used in this session

## Full UI source of truth
Local / artifacts tree: `crates/gearcraft-ui/ui/app.slint` with:
- Sonar palette, ChatMix between Game/Chat, MixerFader + LEDs, streamer dual path
- Camera: Attrezzatura / Effetti tabs, Foto/Video/QR

Also packaged as `gearcraft-full.zip` in project artifacts.

## Restore on a clone
Copy the full `app.slint` from your local build tree, or unpack `gearcraft-full.zip`, then commit:

```bash
cp /path/to/full/app.slint crates/gearcraft-ui/ui/app.slint
git add crates/gearcraft-ui/ui/app.slint
git commit -m "Restore full Slint UI"
git push
```
