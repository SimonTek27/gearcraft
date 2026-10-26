# Spatial audio & volume boost (Sonar-style)

Inspired by SteelSeries Sonar Game tab:

| Control | Meaning |
|---------|---------|
| **Performance ↔ Immersion** | 0 = FPS presence (mids ~2.5 kHz), 100 = cinematic air + softer mids |
| **Near ↔ Far** | Virtual speaker distance (bass tilt) |
| **Volume boost** | Pre-DSP gain −6…+12 dB |
| **Smart volume** | Mild presence leveling |

Applied through the existing WirePlumber filter-chain (`gearcraft.dsp`).

## Presets

Builtin game-oriented presets: `Apex Legends`, `Counter-Strike 2`, `Immersion / Story`.

## IPC

```json
{ "type": "apply_audio_effects", "effects": { "spatial_enabled": true, "spatial_immersion": 20, "volume_boost_db": 1.0 } }
```
