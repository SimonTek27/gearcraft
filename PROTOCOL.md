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
| `list_macros` | — | Saved macros |
| `upsert_macro` | `macro_def` | Create/update macro |
| `delete_macro` | `macro_id` | Delete macro |
| `run_macro` | `macro_id` | Fire macro once |
| `get_chatmix` | — | ChatMix state |
| `set_chatmix` | `balance: i8` | Set balance −100…100 |
| `setup_chatmix` | — | Create Game/Chat sinks |
| `shutdown` | — | Graceful daemon stop |

### Audio presets (JDSP-style)

| Command | Params | Description |
|---------|--------|-------------|
| `list_audio_presets` | — | List saved presets |
| `load_audio_preset` | `preset_id` | Load + apply ChatMix / WirePlumber DSP |
| `save_audio_preset` | `preset: AudioPreset` | Upsert preset |
| `delete_audio_preset` | `preset_id` | Delete preset (+ cascading rules) |
| `list_preset_rules` | — | Device→preset auto-apply rules |
| `set_preset_rule` | `rule: PresetRule` | Add/update rule |
| `delete_preset_rule` | `device_id`, `route_id?` | Remove rule |

See [docs/AUDIO_PRESETS.md](docs/AUDIO_PRESETS.md).
