# Packaging Windows

## Build rapida (PowerShell)

```powershell
./packaging/windows/build.ps1
```

Output: `dist/windows/gearcraft-daemon.exe`, `gearcraft-ui.exe`, icona, README.

## MSI (WiX Toolset 3/4)

1. Installa WiX
2. Genera `.ico` da PNG se manca
3. `candle packaging/windows/gearcraft.wxs && light gearcraft.wixobj`

## Portable ZIP

```powershell
Compress-Archive -Path dist/windows/* -DestinationPath GearCraft-windows-x64.zip
```
