# Automatic app → channel routing

GearCraft classifies running audio clients and moves them onto the matching
virtual sink so Sonar faders control the right content.

| Channel | Sink name | Examples |
|---------|-----------|----------|
| **GAME** | `gearcraft_game` | Steam, Wine, Lutris, Minecraft |
| **CHAT** | `gearcraft_chat` | Discord, Teamspeak, Zoom, Slack, Telegram |
| **MEDIA** | `gearcraft_media` | Haruna, VLC, mpv, Spotify, Strawberry |
| **AUX** | `gearcraft_aux` | OBS |

## How it works

1. **Setup sinks** (Sonar → Setup sinks) creates the four null sinks via `pactl`.
2. Every **2 seconds** the daemon lists `pactl list sink-inputs`.
3. Each stream is matched against rules (`application.process.binary` /
   `application.name`, case-insensitive substring).
4. Matching streams are moved with `pactl move-sink-input <id> <sink>`.

## Custom rules

File: `~/.config/gearcraft/app_routes.json`

```json
{
  "enabled": true,
  "rules": [
    { "match_pattern": "discord", "channel": "chat", "note": "voice" },
    { "match_pattern": "haruna", "channel": "media", "note": "video" },
    { "match_pattern": "mygame", "channel": "game", "note": "" }
  ]
}
```

`channel` values: `game`, `chat`, `media`, `aux`, `ignore`.

On first run the daemon writes the built-in defaults into this file; edit and
save — rules reload about every minute.

## Requirements

- `pactl` (pipewire-pulse)
- Sinks created (`setup_chatmix` / Setup sinks)

Disable auto-routing:

```json
{ "enabled": false, "rules": [] }
```
