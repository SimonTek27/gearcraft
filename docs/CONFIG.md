# Configuration & persistence

GearCraft stores user data under the XDG config directory:

```
$XDG_CONFIG_HOME/gearcraft/     # or ~/.config/gearcraft
├── macros.json                 # macro definitions
└── settings.json               # ChatMix balance + future prefs
```

## macros.json

Array of macro objects (same shape as the IPC `MacroDef`):

```json
[
  {
    "id": "demo-clip",
    "name": "Clip highlight",
    "device_id": null,
    "trigger": "Mouse button 4",
    "actions": [{ "key": "F8", "delay_ms": 0 }],
    "enabled": true
  }
]
```

- Loaded at daemon start.
- Written on every **upsert** / **delete** from the UI or IPC.
- If the file is missing, demo macros are created and saved once.

## settings.json

```json
{
  "version": 1,
  "chatmix": {
    "balance": 0,
    "game_volume": 100,
    "chat_volume": 100
  }
}
```

- Loaded at daemon start; ChatMix volumes are re-applied if Game/Chat sinks exist.
- Updated when IPC `set_chatmix` succeeds (state is saved even if audio apply fails).

## Environment

| Variable | Effect |
|----------|--------|
| `XDG_CONFIG_HOME` | Base for config dir |
| `HOME` | Fallback `~/.config/gearcraft` |
| `GEARCRAFT_DRY=1` | HID dry-run (unrelated to config) |

Writes use a temp file + rename for basic atomicity.
