#!/usr/bin/env bash
# Install GearCraft daemon + udev rules (run as root)
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PREFIX="${PREFIX:-/usr}"

echo "==> Building gearcraft-daemon"
cd "$ROOT"
cargo build -p gearcraft-daemon --release ${FEATURES:+--features "$FEATURES"}

echo "==> Installing binary to $PREFIX/bin"
install -Dm755 target/release/gearcraft-daemon "$PREFIX/bin/gearcraft-daemon"

echo "==> Installing udev rules"
install -Dm644 assets/udev/99-gearcraft.rules /etc/udev/rules.d/99-gearcraft.rules
udevadm control --reload-rules
udevadm trigger || true

echo "==> Installing systemd unit"
install -Dm644 packaging/gearcraft-daemon.service /etc/systemd/system/gearcraft-daemon.service
systemctl daemon-reload

echo ""
echo "Done. Enable and start with:"
echo "  sudo systemctl enable --now gearcraft-daemon"
echo ""
echo "Optional: add your user to plugdev and re-login:"
echo "  sudo usermod -aG plugdev \$USER"
echo ""
echo "Live USB build: FEATURES=hidapi sudo -E ./packaging/install.sh"
