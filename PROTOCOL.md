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

## Tauri commands (UI side)

Exposed from `src-tauri` and callable via `invoke()`:

- `get_devices()`
- `get_device_status(device_id)`
- `set_color(device_id, zone, r, g, b)`
- `ping_daemon()`

These are thin wrappers that speak the Unix-socket protocol above.

## Macros

| Command | Params | Description |
|---------|--------|-------------|
| `list_macros` | — | All macro definitions |
| `upsert_macro` | `macro_def` | Create/update |
| `delete_macro` | `macro_id` | Remove |
| `run_macro` | `macro_id` | Execute (uinput stub logs keys) |

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
| `set_chatmix` | `balance: -100..=100` | Updates volumes; PipeWire apply TBD |

### setup_chatmix

Creates `gearcraft_game` and `gearcraft_chat` null sinks via `pactl` (pipewire-pulse).

`set_chatmix` applies volumes through `wpctl set-volume` or `pactl set-sink-volume` when sinks are detected.
