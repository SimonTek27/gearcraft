# GearCraft ChatMix (PipeWire)

Software game/chat balance on PipeWire — works with any headset or default audio path.

## How it works

1. Two sinks: **Game** and **Chat** (null sinks or existing devices).
2. The daemon sets their volumes from the Sonar slider (`balance` −100…+100).
3. You route applications to the right sink (pavucontrol, Helvum, EasyEffects).

## Backend detection

| Tool | Package | Role |
|------|---------|------|
| `wpctl` | wireplumber | Preferred volume control |
| `pactl` | pipewire-pulse | Fallback + null-sink creation |

## Quick setup

```bash
# Create virtual sinks (also: Sonar → "Setup Game/Chat sinks")
pactl load-module module-null-sink sink_name=gearcraft_game \
  sink_properties=device.description=GearCraft_Game
pactl load-module module-null-sink sink_name=gearcraft_chat \
  sink_properties=device.description=GearCraft_Chat

# Optional: hear both on the default output
export GEARCRAFT_CHATMIX_LOOPBACK=1
# then run setup again, or:
pactl load-module module-loopback source=gearcraft_game.monitor latency_msec=20
pactl load-module module-loopback source=gearcraft_chat.monitor latency_msec=20
```

### Custom sink names

```bash
export GEARCRAFT_GAME_SINK=56          # wpctl id
export GEARCRAFT_CHAT_SINK=my_chat     # name substring or pactl name
```

## IPC

- `get_chatmix` – state + backend info  
- `set_chatmix { balance }` – update state **and** apply volumes  
- `setup_chatmix` – create `gearcraft_game` / `gearcraft_chat` null sinks  

## App routing tips

- Games → **GearCraft_Game**
- Discord / TeamSpeak → **GearCraft_Chat**
- Use WirePlumber rules or `pw-loopback` for permanent links
