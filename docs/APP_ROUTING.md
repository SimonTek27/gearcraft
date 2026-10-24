# Automatic app → channel routing

GearCraft **automatically** classifies running audio clients and moves them onto
the matching virtual sink. No configuration file is required.

| Channel | Sink name | Examples |
|---------|-----------|----------|
| **GAME** | `gearcraft_game` | Steam, Wine, Lutris, Minecraft |
| **CHAT** | `gearcraft_chat` | Discord, Teamspeak, Zoom, Slack, Telegram |
| **MEDIA** | `gearcraft_media` | Haruna, VLC, mpv, Spotify, Strawberry |
| **AUX** | `gearcraft_aux` | OBS |

## How it works

1. **Setup sinks** (Sonar → Setup sinks) creates the four null sinks via `pactl`.
2. Every **2 seconds** the daemon lists `pactl list sink-inputs`.
3. Each stream is matched against **built-in** rules (`application.process.binary` /
   `application.name`, case-insensitive substring).
4. Matching streams are moved with `pactl move-sink-input <id> <sink>`.

Built-in patterns include Discord → chat, Haruna/VLC/Spotify → media, Steam/Wine → game, OBS → aux.

## Requirements

- `pactl` (pipewire-pulse)
- Sinks created (`setup_chatmix` / Setup sinks)
