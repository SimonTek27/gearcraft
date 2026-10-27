# Build GearCraft for Windows (run in PowerShell from repo root)
$ErrorActionPreference = "Stop"
$Target = if ($env:GEARCRAFT_TARGET) { $env:GEARCRAFT_TARGET } else { "x86_64-pc-windows-msvc" }

Write-Host "Target: $Target"
rustup target add $Target

cargo build --release -p gearcraft-daemon --target $Target
cargo build --release -p gearcraft-ui --target $Target

$Out = "dist/windows"
New-Item -ItemType Directory -Force -Path $Out | Out-Null
Copy-Item "target/$Target/release/gearcraft-daemon.exe" $Out
Copy-Item "target/$Target/release/gearcraft-ui.exe" $Out
if (Test-Path "assets/icons/gearcraft-256.png") {
  Copy-Item "assets/icons/gearcraft-256.png" "$Out/gearcraft.png"
}
Copy-Item "docs/WINDOWS.md" "$Out/README-WINDOWS.md" -ErrorAction SilentlyContinue

Write-Host "Artifacts in $Out"
Get-ChildItem $Out
