# Razer addon

OpenRazer-compatible HID control for Chroma mice and keyboards.

## Capabilities

| Action | Notes |
|--------|--------|
| `set_color` zones `logo`, `scroll`, `backlight`, `all` | Classic mice |
| Matrix static | Keyboards / Firefly-class mats |
| `apply_profile` with `colors` / `color` JSON | Same as SteelSeries |

## Protocol

90-byte feature report (`0x5A`), CRC = XOR of bytes `[2..88)` — same frame as
OpenRazer / OpenRGB.

## Requirements

```bash
cargo run -p gearcraft-daemon --features hidapi
sudo cp assets/udev/99-gearcraft.rules /etc/udev/rules.d/
sudo udevadm control --reload-rules && sudo udevadm trigger
```

OpenRazer kernel modules are **not** required — GearCraft talks HID directly.

## Zones

- **Mice**: logo, scroll (wheel), backlight, all
- **Keyboards**: matrix static via zone `all` / `main`

## Limitations

- Unknown PIDs still bind if VID is `0x1532`
- Wireless dongles may expose multiple interfaces — one path is selected
- Advanced effects (wave, reactive) not yet exposed
