# GearCraft IPC Protocol

Communication between **gearcraft-ui** (Slint) and **gearcraft-daemon** uses a **Unix domain socket**.

## Socket path

```
$XDG_RUNTIME_DIR/gearcraft.sock
# fallback
/tmp/gearcraft.sock
```

## Framing

Every message is **length-prefixed**:

```
[u32 little-endian length][UTF-8 JSON body]
```

Max body size: 1 MiB.

## Request

```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "cmd": {
    "name": "get_devices"
  }
}
```

`cmd` is a tagged enum (`name` + optional `params`).

## Response

```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "ok": true,
  "data": [ ... ]
}
```

or

```json
{
  "id": "...",
  "ok": false,
  "error": "device not found: xyz"
}
```

## Commands

| Command | Params | Description |
|---------|--------|-------------|
| `ping` | — | Health check → `"pong"` |
| `get_devices` | — | List all known devices |
| `get_device_status` | `device_id: string` | Status + battery + details |
| `apply_profile` | `device_id`, `profile: object` | Apply full profile JSON |
| `set_color` | `device_id`, `zone: string`, `color: {r,g,b}` | Quick RGB set |
| `list_addons` | — | Loaded brand addons |
| `shutdown` | — | Graceful daemon stop |

### Mouse (libratbag / Piper)

| Command | Params | Description |
|---------|--------|-------------|
| `get_mouse_info` | `device_id` | DPI range + active profile via `ratbagctl` |
| `set_dpi` | `device_id`, `dpi` | Set DPI and commit to the device |

### Lighting (official backends first, raw HID as fallback)

| Command | Params | Description |
|---------|--------|-------------|
| `set_color` | `device_id`, `zone`, `color` | Static colour → `official` \| `hid` backend |
| `set_effect` | `device_id`, `zone`, `effect`, `color?` | Named effect → `official` \| `hid` backend |
| `prism_sync` | `effect`, `zone`, `color?` | Same effect on every OpenRGB/OpenRazer device |

`effect` ∈ `static | rainbow | breathe | spectrum | wave | off`.
See [docs/OFFICIAL_DRIVERS.md](docs/OFFICIAL_DRIVERS.md).

### Microphone FX

| Command | Params | Description |
|---------|--------|-------------|
| `get_mic_fx` | — | Snapshot (`MicFxState`) read via `pactl` |
| `set_mic_fx` | `state: MicFxState` | Apply gain/mute/high-pass/gate to the default source |

See [docs/MIC_FX.md](docs/MIC_FX.md).

### DSP (live effects)

| Command | Params | Description |
|---------|--------|-------------|
| `apply_audio_effects` | `effects: AudioEffects` | Rebuild the WirePlumber filter chain now |

### Screen capture

| Command | Params | Description |
|---------|--------|-------------|
| `capture_screenshot` | `mode` (`full \| window \| region`), `delay_secs`, `clipboard` | Returns `{ path, backend, message }` |

See [docs/SCREENSHOT.md](docs/SCREENSHOT.md).

### Example: get_devices

**Request**
```json
{
  "id": "…",
  "cmd": { "name": "get_devices" }
}
```

**Response data**
```json
[
  {
    "id": "ss-rival3-001",
    "name": "Rival 3",
    "vendor": "SteelSeries",
    "kind": "mouse",
    "vendor_id": 4152,
    "product_id": 6180,
    "path": "/dev/hidraw0"
  }
]
```

### Example: set_color

**Request**
```json
{
  "id": "…",
  "cmd": {
    "name": "set_color",
    "params": {
      "device_id": "ss-rival3-001",
      "zone": "logo",
      "color": { "r": 0, "g": 231, "b": 160 }
    }
  }
}
```

## UI side

The UI is a Slint binary (`crates/gearcraft-ui`), not Tauri. Its client
(`crates/gearcraft-ui/src/ipc.rs`) speaks the Unix-socket protocol above with
blocking `connect → write → read` calls; long-running commands such as
`capture_screenshot` are dispatched on a worker thread and marshalled back to
the UI event loop.

## Macros

| Command | Params | Description |
|---------|--------|-------------|
| `list_macros` | — | All macro definitions |
| `upsert_macro` | `macro_def` | Create/update |
| `delete_macro` | `macro_id` | Remove |
| `run_macro` | `macro_id` | Execute via uinput (`uinput_macros::play_macro`) |

```json
{
  "id": "demo-clip",
  "name": "Clip highlight",
  "trigger": "Mouse button 4",
  "actions": [{ "key": "F8", "delay_ms": 0 }],
  "enabled": true
}
```

## ChatMix

| Command | Params | Description |
|---------|--------|-------------|
| `get_chatmix` | — | `{ balance, game_volume, chat_volume }` |
| `set_chatmix` | `balance: -100..=100` | Updates volumes and applies them through `wpctl` / `pactl` |

### setup_chatmix

Creates `gearcraft_game` and `gearcraft_chat` null sinks via `pactl` (pipewire-pulse).

`set_chatmix` applies volumes through `wpctl set-volume` or `pactl set-sink-volume` when sinks are detected.


### Audio presets (JDSP-style)

| Command | Params | Description |
|---------|--------|-------------|
| `list_audio_presets` | — | List saved presets |
| `load_audio_preset` | `preset_id` | Load + apply ChatMix / effects state |
| `save_audio_preset` | `preset: AudioPreset` | Upsert preset |
| `delete_audio_preset` | `preset_id` | Delete preset (+ cascading rules) |
| `list_preset_rules` | — | Device→preset auto-apply rules |
| `set_preset_rule` | `rule: PresetRule` | Add/update rule |
| `delete_preset_rule` | `device_id`, `route_id?` | Remove rule |

See [docs/AUDIO_PRESETS.md](docs/AUDIO_PRESETS.md).

JamesDSP/JDSP4Linux `.conf` imports use `ImportJamesDspPreset { path, apply }`
and `ImportJamesDspDir { path, apply_first }` — see
[docs/JAMESDSP_IMPORT.md](docs/JAMESDSP_IMPORT.md).


## Device Manager

| Command | Params | Description |
|---------|--------|-------------|
| `GetDeviceManager` | — | Outputs + policies |
| `SetDeviceManager` | `state` | Full replace + apply |
| `SetChannelDevicePolicy` | `policy` | One channel |
| `LinkAllDevices` | `device_id?`, `enabled` | Same device on all channels |
| `ApplyDeviceManager` | — | Re-apply loopbacks |

See [docs/DEVICE_MANAGER.md](docs/DEVICE_MANAGER.md).
