# Official driver backends

GearCraft prefers **system drivers** over raw HID when available.

## Priority

1. **OpenRazer** (Razer devices) — D-Bus `org.razer` via `busctl --user`
2. **OpenRGB** (multi-vendor, including SteelSeries) — `openrgb` CLI
3. **Raw HID** — built-in Razer / SteelSeries protocols (fallback)

## Install

### Arch
```bash
sudo pacman -S openrazer-daemon openrazer-driver-dkms openrgb
sudo gpasswd -a $USER plugdev
# reboot or re-login; start daemon:
systemctl --user enable --now openrazer-daemon
```

### Debian / Ubuntu
```bash
sudo apt install openrazer-daemon openrazer-driver-dkms openrgb
sudo gpasswd -a $USER plugdev
```

Start OpenRGB in SDK server mode (optional GUI) so the CLI can attach.

## Verify

```bash
# OpenRazer
busctl --user status org.razer
busctl --user call org.razer /org/razer/devices razer.devices getDevices

# OpenRGB
openrgb -l
```

## Prism / Engine

Applying a colour or effect from the Prism page uses this stack automatically.
The IPC response includes `"backend": "official" | "hid"`.
