# Sonar Device Manager

Maps each Sonar channel (Game / Chat / Media / Aux) to a **physical** PipeWire
sink, with priority lists and automatic backup — same idea as SteelSeries
“Quick Setup Your Audio Devices”.

## Model

1. Apps play into virtual sinks `gearcraft_{game,chat,media,aux}` (Setup sinks).
2. Device Manager loads `module-loopback` from `virtual.monitor` → chosen sink.
3. **Priority**: first available non-excluded device in the list is used.
4. **Backup automatic**: every 3s, if the active sink disappears, re-pick next.
5. **Link all**: same primary device forced on every channel.

## IPC

| Command | Effect |
|---------|--------|
| `GetDeviceManager` | `{ state, devices[] }` |
| `SetDeviceManager { state }` | Replace config + apply |
| `SetChannelDevicePolicy { policy }` | One channel priority/exclude |
| `LinkAllDevices { device_id?, enabled }` | Link / unlink |
| `ApplyDeviceManager` | Re-apply loopbacks |

Persisted at `~/.config/gearcraft/device_manager.json`.
