# Game preset auto-switch (Quick Set)

When a known game process is detected, GearCraft applies the matching audio
preset automatically — similar to SteelSeries Sonar Quick Set.

## Detection

Every 4s: `pactl list sink-inputs` + `/proc/*/comm`.

## Map

| Process | Preset |
|---------|--------|
| r5apex, apex… | Apex Legends |
| cs2, csgo | Counter-Strike 2 |
| valorant, overwatch, fortnite | CS2-style |
| eldenring, cyberpunk, … | Immersion / Story |

Edit `GAME_MAP` in `game_presets.rs` to extend.
