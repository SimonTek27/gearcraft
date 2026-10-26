# Microphone FX

Sonar-style mic controls for the default PipeWire/Pulse source.

| Control | Backend |
|---------|---------|
| Gain 0–150% | `pactl set-source-volume` |
| Mute | `pactl set-source-mute` |
| High-pass (~100 Hz) | UI state + log (DSP extension later) |
| Noise gate | UI state (strength stored) |

## IPC

- `GetMicFx`
- `SetMicFx { state }`

## UI

Sonar → **MICROPHONE** → **Applica Mic**.
