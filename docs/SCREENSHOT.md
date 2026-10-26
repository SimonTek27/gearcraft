# Screen capture (Spectacle-style)

Capture modes from the **Engine** page:

| Mode | Description |
|------|-------------|
| Full screen | Entire display |
| Active window | Focused window (when backend supports it) |
| Region | Interactive selection |

Images are saved under `~/Pictures/GearCraft/` and copied to the clipboard when possible.

## Backends (first available)

1. **spectacle** (KDE) — preferred  
2. **gnome-screenshot**  
3. **grim** (+ **slurp** for region) — Wayland  
4. **maim** / **scrot** — X11  

```bash
# KDE
sudo pacman -S spectacle
# Wayland generic
sudo pacman -S grim slurp wl-clipboard
```
