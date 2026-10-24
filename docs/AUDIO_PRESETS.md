# Audio presets (JDSP4Linux replacement path)

GearCraft stores **named audio presets** and **device→preset rules** the same way
[JDSP4Linux](https://github.com/Audio4Linux/JDSP4Linux) does (`--list-presets`,
`--load-preset`, `--save-preset`, `--delete-preset`, `--list-preset-rules`, …).

Goal: replace JamesDSP for day-to-day preset workflow on PipeWire, while the
actual DSP engine (EQ/crossfeed/convolver) is applied via WirePlumber (`wpexec`).

## Storage

```
~/.config/gearcraft/
├── audio_presets.json   # array of AudioPreset
├── preset_rules.json    # array of PresetRule
└── dsp/
    ├── gearcraft-dsp.lua    # generated wpexec script
    ├── filter-chain.json    # SPA-JSON module args
    └── wpexec.pid           # loader PID
```

### AudioPreset

| Field | Meaning |
|-------|---------|
| `id` | Stable id (slug) |
| `name` | Display name |
| `effects` | EQ bands, bass boost, compressor, reverb, crossfeed, wideness, limiter, bypass |
| `chatmix` | Optional ChatMix snapshot applied on load |
| `notes` | Free text |

EQ is a **10-band graphic** list (`freq_hz` + `gain_db`) portable to FIR/IIR backends later.

### PresetRule

| Field | Meaning |
|-------|---------|
| `device_id` | GearCraft device id or sink name |
| `device_name` | Label |
| `route_id` | Optional route (headphone vs speaker) |
| `preset_id` | Preset to auto-load |

## IPC (aligned with JDSP CLI)

| Command | JDSP equivalent |
|---------|-----------------|
| `list_audio_presets` | `--list-presets` |
| `load_audio_preset` | `--load-preset <name>` |
| `save_audio_preset` | `--save-preset <name>` |
| `delete_audio_preset` | `--delete-preset <name>` |
| `list_preset_rules` | `--list-preset-rules` |
| `set_preset_rule` | `--set-preset-rule` |
| `delete_preset_rule` | `--delete-preset-rule` |

### Load behaviour today

1. Resolve preset by id.
2. If `chatmix` is set → update daemon state, persist, `apply_volumes` on Game/Chat sinks.
3. **WirePlumber DSP bridge** (`dsp_pw`): `wpexec` loads filter-chain (no PipeWire restart).
4. Return payload with `dsp_engine: "wireplumber-wpexec"`.

### WirePlumber EQ + crossfeed

Backend: **`wpexec`** loads `libpipewire-module-filter-chain`.

Virtual sink: **GearCraft DSP** (`gearcraft.dsp`) with `filter.smart=true`.

| Effect | Implementation |
|--------|----------------|
| Graphic EQ | Builtin `bq_peaking` / `bq_lowshelf` (10 bands) |
| Bass boost | Extra `bq_lowshelf` @ 80 Hz |
| Preamp / limiter headroom | `bq_highshelf` @ 0 Hz |
| Crossfeed | LADSPA **bs2b** if installed, else soft stereo mixer |

```bash
# Requires wireplumber (wpexec + wpctl)
wpctl status | grep -i gearcraft
wpctl set-default <id>

# Auto-set default after apply:
GEARCRAFT_DSP_DEFAULT=1 cargo run -p gearcraft-daemon
```

### Built-in presets

- **Flat** — neutral + balanced ChatMix
- **Bass Boost** — low-shelf EQ + bass flag
- **Vocal Clarity** — presence lift + compressor flag
- **Crossfeed** — headphone stereo image
- **Game FPS** — footsteps emphasis + slight game ChatMix bias

## UI

Sonar page → **PRESET AUDIO** panel: list, load, delete, save current ChatMix as
`Custom` / `Session`. Active preset name is shown in the header.

## Roadmap toward full JDSP replacement

- [x] Preset CRUD + disk persistence
- [x] Device rules data model + IPC
- [x] ChatMix apply on load
- [x] WirePlumber filter-chain bridge for EQ + crossfeed (`dsp_pw`)
- [ ] Auto-apply rules on headset hotplug
- [ ] AutoEQ import
- [ ] Convolver IR attach
