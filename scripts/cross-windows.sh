#!/usr/bin/env bash
# Cross-compile GearCraft for Windows GNU from Linux
set -euo pipefail
TARGET="${GEARCRAFT_TARGET:-x86_64-pc-windows-gnu}"
rustup target add "$TARGET"
cargo build --release -p gearcraft-daemon --target "$TARGET"
cargo build --release -p gearcraft-ui --target "$TARGET"
mkdir -p dist/windows
cp "target/$TARGET/release/gearcraft-daemon.exe" dist/windows/ || true
cp "target/$TARGET/release/gearcraft-ui.exe" dist/windows/ || true
echo "Done — see dist/windows (if linker available)"
