# Priorità implementate (2026-10-26)

## 1. DPI mouse (libratbag / Piper)
- Backend: `ratbagctl` (`drivers/ratbag.rs`)
- IPC: `GetMouseInfo`, `SetDpi`
- UI Engine: preset 400–6400 DPI

```bash
sudo pacman -S piper libratbag
systemctl --user enable --now ratbagd   # or system service per distro
```

## 2. PrismSync
- IPC: `PrismSync { effect, zone, color }`
- Applica via **OpenRGB** (tutti i device) + **OpenRazer** (tutti i serial)
- UI Prism: **Sync tutti**

## 3. Convolver
- Campi `convolver_enable`, `convolver_file`, `convolver_mix` in `AudioEffects`
- Path IR salvato nei preset; runtime annota (partitioned convolver pieno → EasyEffects/JDSP)

## 4. Mic RNNoise
- `MicFxState.rnnoise`
- Rileva LADSPA rnnoise; fallback `module-echo-cancel`

```bash
# community package often:
# noise-suppression-for-voice / rnnoise
```

## 5. Packaging AUR + udev
- `packaging/aur/PKGBUILD`
- `packaging/udev/99-gearcraft.rules` (SteelSeries 1038, Razer 1532)
- `packaging/gearcraft-daemon.service`
