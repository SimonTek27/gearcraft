# GearCraft su Windows

Versione Windows del hub (UI Slint + daemon). Il protocollo IPC è lo stesso (JSON length-prefixed); cambia solo il trasporto.

## Stato

| Area | Linux | Windows |
|------|-------|---------|
| UI Slint | ✅ | ✅ (MSVC o GNU) |
| Daemon | ✅ | ✅ (base) |
| IPC | Unix socket `$XDG_RUNTIME_DIR/gearcraft.sock` | Named pipe `\\\\.\\pipe\\gearcraft` |
| Audio Sonar / EQ / routing | PipeWire + WirePlumber | **Stub** — WASAPI/APO in roadmap |
| HID / RGB | hidapi + OpenRazer/OpenRGB | hidapi (hid.dll) + OpenRGB CLI se installato |
| Camera V4L2 | ✅ | Non disponibile (modulo Linux-only) |

## Build

```powershell
rustup target add x86_64-pc-windows-msvc
./packaging/windows/build.ps1
```

IPC Windows: `\\.\pipe\gearcraft`
