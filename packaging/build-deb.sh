#!/usr/bin/env bash
# Build a simple .deb for gearcraft-daemon (user-space, no root needed at runtime
# beyond udev rules).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
VERSION="${VERSION:-0.1.0}"
ARCH="${ARCH:-amd64}"
PKG="gearcraft-daemon_${VERSION}_${ARCH}"
OUT="$ROOT/packaging/${PKG}"

rm -rf "$OUT"
mkdir -p "$OUT/DEBIAN" "$OUT/usr/bin" "$OUT/usr/lib/systemd/user" \
         "$OUT/usr/lib/udev/rules.d" "$OUT/usr/share/doc/gearcraft-daemon"

# Binary
cargo build -p gearcraft-daemon --release --manifest-path "$ROOT/Cargo.toml"
install -m755 "$ROOT/target/release/gearcraft-daemon" "$OUT/usr/bin/gearcraft-daemon"

# systemd user unit
install -m644 "$ROOT/packaging/gearcraft-daemon.service" \
  "$OUT/usr/lib/systemd/user/gearcraft-daemon.service"

# udev
install -m644 "$ROOT/assets/udev/99-gearcraft.rules" \
  "$OUT/usr/lib/udev/rules.d/99-gearcraft.rules"

# docs
cp "$ROOT/LICENSE" "$OUT/usr/share/doc/gearcraft-daemon/copyright"
echo "GearCraft daemon $VERSION" > "$OUT/usr/share/doc/gearcraft-daemon/README"

# control
cat > "$OUT/DEBIAN/control" <<EOF
Package: gearcraft-daemon
Version: $VERSION
Section: utils
Priority: optional
Architecture: $ARCH
Depends: libc6, libudev1
Maintainer: GearCraft Contributors
Description: Open-source peripheral daemon for SteelSeries and friends on Linux
 GearCraft provides RGB, DPI, macros and ChatMix without proprietary software.
EOF

cat > "$OUT/DEBIAN/postinst" <<'EOF'
#!/bin/sh
set -e
udevadm control --reload-rules || true
udevadm trigger || true
exit 0
EOF
chmod 755 "$OUT/DEBIAN/postinst"

dpkg-deb --build "$OUT" "$ROOT/packaging/${PKG}.deb"
echo "Built $ROOT/packaging/${PKG}.deb"
