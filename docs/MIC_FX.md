# Microphone FX (real DSP)

Pipeline:

```
hardware mic → high-pass → low-pass → gate → compressor → gearcraft.mic
```

| Effect | Implementation |
|--------|----------------|
| High-pass | `bq_highpass` (40–300 Hz) |
| Low-pass | `bq_lowpass` (3–16 kHz, default 10 kHz) |
| Gate | LADSPA `gate_1410` / soft fallback |
| Compressor | LADSPA `sc4` (`swh-plugins`) |

```bash
sudo pacman -S swh-plugins wireplumber
```

UI: Sonar → MICROPHONE → HPF / LPF / Gate / Comp → **Applica Mic**
