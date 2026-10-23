# GearCraft ChatMix (PipeWire)

Software game/chat balance without SteelSeries GG.

1. Create sinks: `gearcraft_game` / `gearcraft_chat` (Sonar UI or pactl).
2. Route apps in pavucontrol/Helvum.
3. Move the ChatMix slider — daemon applies volumes via wpctl or pactl.

Env: `GEARCRAFT_GAME_SINK`, `GEARCRAFT_CHAT_SINK`, `GEARCRAFT_CHATMIX_LOOPBACK=1`.
