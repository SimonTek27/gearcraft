# Hotplug & auto-apply presets

The daemon watches USB via `udevadm monitor --udev --subsystem-match=usb`.

On **add**:
1. Registry refresh + addon bind (SteelSeries / Razer)
2. Lookup `PresetRule` for the device id (or name substring)
3. Apply matching audio preset (ChatMix volumes + PipeWire DSP)

If `udevadm` is missing, falls back to a 5s poll.

## Rules

Set rules through IPC (`set_preset_rule`) or `~/.config/gearcraft/preset_rules.json`:

```json
[
  { "device_id": "arctis", "route_id": "", "preset_id": "bass-boost" }
]
```

`device_id` may be a full device id or a substring of the device name.
