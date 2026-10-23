#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PREFIX="${PREFIX:-/usr}"
cd "$ROOT"
cargo build -p gearcraft-daemon --release ${FEATURES:+--features "$FEATURES"}
install -Dm755 target/release/gearcraft-daemon "$PREFIX/bin/gearcraft-daemon"
install -Dm644 assets/udev/99-gearcraft.rules /etc/udev/rules.d/99-gearcraft.rules
udevadm control --reload-rules || true
install -Dm644 packaging/gearcraft-daemon.service /etc/systemd/system/gearcraft-daemon.service
systemctl daemon-reload || true
echo "Enable with: systemctl enable --now gearcraft-daemon"
