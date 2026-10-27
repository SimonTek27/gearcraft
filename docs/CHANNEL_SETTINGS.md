# Channel settings card (Sonar-style)

Applied on **GitHub `main`**.

## UI
- **⚙** next to each channel label in the Sonar mixer
- Modal card:
  - Primary output device
  - Backup device
  - Hotkeys: Volume +, Volume −, Mute

## Model (`ChannelDevicePolicy`)
| Field | Role |
|-------|------|
| `primary_device_id` | Preferred sink |
| `backup_device_id` | Failover |
| `hotkey_volume_*` / `hotkey_mute` | Stored chords |
| `priority` | Synced via `sync_priority_from_primary_backup()` |

## Files
- `crates/gearcraft-core/src/lib.rs` ✅ on main
- `crates/gearcraft-daemon/src/device_manager.rs` ✅ on main
- `crates/gearcraft-ui/ui/app.slint` + `src/main.rs` — channel gear + modal handlers
