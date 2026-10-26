# Game preset auto-switch (Quick Set)

When a known game process is detected, GearCraft applies the matching audio
preset (EQ + spatial + boost) automatically — similar to SteelSeries Sonar
“Quick Set” / game configurations.

## Detection

Every 4 seconds the daemon inspects:

1. `pactl list sink-inputs` → `application.process.binary` / `application.name`
2. `/proc/*/comm` process names

## Built-in map

| Process substring | Preset |
|-------------------|--------|
| r5apex, apex… | Apex Legends |
| cs2, csgo | Counter-Strike 2 |
| valorant, overwatch, fortnite | CS2-style competitive |
| eldenring, cyberpunk, witcher3, … | Immersion / Story |

## Extend

Edit `crates/gearcraft-daemon/src/game_presets.rs` (`GAME_MAP`) or save a custom
preset id and add a mapping entry.

## Notes

- The last applied game preset is remembered for the daemon session (no re-apply spam).
- Manual preset load from the UI still works and is not blocked.
