# Push status (2026-10-27)

Large `app.slint` (~193KB) needs a follow-up restore if the tree shows PLACEHOLDER.

**Local source of truth:** full project tree under artifacts/gearcraft.

Already on main in this push series: Camera docs, UI palette, ChatMix/streamer docs, Windows packaging, Cargo target gates, audio_windows stub.

To restore UI from a machine with the full tree:

```bash
git clone https://github.com/SimonTek27/gearcraft
# copy crates/gearcraft-ui/ui/app.slint from local full tree
git add crates/gearcraft-ui/ui/app.slint crates/gearcraft-ui/src/
git commit -m "Restore full Slint UI"
git push
```
