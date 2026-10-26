# Microphone FX (real DSP)

High-pass and noise gate run through a **WirePlumber** `libpipewire-module-filter-chain`
instance (separate from the output EQ chain).

## Pipeline

```
hardware mic  →  bq_highpass  →  LADSPA gate (or soft fallback)  →  gearcraft.mic (source)
```

- Node name: `gearcraft.mic` (“GearCraft Mic”)
- Script: `~/.config/gearcraft/dsp/gearcraft-mic.lua`
- PID: `~/.config/gearcraft/dsp/wpexec-mic.pid`

## High-pass

Builtin `bq_highpass` (Q ≈ 0.707). Frequency from UI (`highpass_hz`, 40–300 Hz).

## Noise gate

1. LADSPA `gate_1410` (`swh-plugins`)
2. LADSPA `sc4` compressor as soft gate
3. Fallback: steeper HP + low-shelf cut

```bash
sudo pacman -S swh-plugins wireplumber   # Arch
sudo apt install swh-plugins wireplumber  # Debian/Ubuntu
```

## Gain / mute

`pactl` on `gearcraft.mic`. Default source switched to filtered mic when DSP is on.

## UI

Sonar → **MICROPHONE** → **Applica Mic**
