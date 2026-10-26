# Import JamesDSP presets (`.conf`)

GearCraft can import [JDSP4Linux](https://github.com/Audio4Linux/JDSP4Linux) preset files
from `~/.config/jamesdsp/presets/*.conf`.

## Mapped fields

| JamesDSP | GearCraft |
|----------|-----------|
| `tone_enable` + `tone_eq` | EQ bands |
| `graphiceq_param` | EQ bands (GraphicEQ syntax) |
| `bass_enable` / `bass_maxgain` | bass boost |
| `crossfeed_enable` / `crossfeed_bs2b_feed` | crossfeed strength |
| `compander_enable` | compressor (approx) |
| `stereowide_*` | wideness |
| `master_limthreshold` / `master_postgain` | limiter + volume boost |

**Not mapped** (noted in preset notes): convolver IR, ViPER-DDC, liveprog, full reverb, tube.

## IPC

```json
{
  "cmd": {
    "ImportJamesDspPreset": {
      "path": "/home/you/.config/jamesdsp/presets/Clear.conf",
      "apply": true
    }
  }
}
```

Import entire folder:

```json
{
  "cmd": {
    "ImportJamesDspDir": {
      "path": "/home/you/.config/jamesdsp/presets",
      "apply_first": false
    }
  }
}
```

## CLI helper

```bash
# after daemon is running, from a small client or future UI button:
# copy presets then import dir
ls ~/.config/jamesdsp/presets/*.conf
```

Imported presets appear in Sonar → Preset list with id prefix `jdsp-`.
