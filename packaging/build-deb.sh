#!/usr/bin/env bash
# Build a simple .deb without full dpkg-buildpackage (no rustc version fights).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
VERSION=0.1.0
ARCH="$(dpkg --print-architecture 2>/dev/null || echo amd64)"
PKG=gearcraft-daemon_${VERSION}-1_${ARCH}
DEST="/tmp/${PKG}"

echo "==> Building release binary"
cargo build --release -p gearcraft-daemon ${FEATURES:+--features "$FEATURES"}

rm -rf "$DEST"
mkdir -p "$DEST/DEBIAN" \
         "$DEST/usr/bin" \
         "$DEST/lib/udev/rules.d" \
         "$DEST/lib/systemd/system" \
         "$DEST/usr/share/doc/gearcraft-daemon"

install -m755 target/release/gearcraft-daemon "$DEST/usr/bin/"
install -m644 assets/udev/99-gearcraft.rules "$DEST/lib/udev/rules.d/"
install -m644 packaging/gearcraft-daemon.service "$DEST/lib/systemd/system/"
install -m644 LICENSE "$DEST/usr/share/doc/gearcraft-daemon/copyright"
gzip -9c packaging/debian/changelog > "$DEST/usr/share/doc/gearcraft-daemon/changelog.Debian.gz"

cat > "$DEST/DEBIAN/control" << CTRL
Package: gearcraft-daemon
Version: ${VERSION}-1
Section: utils
Priority: optional
Architecture: ${ARCH}
Maintainer: GearCraft Contributors <dev@gearcraft.local>
Depends: adduser
Recommends: libhidapi-hidraw0
Description: Open-source multi-brand peripheral daemon
 Alternative to SteelSeries GG on Linux (Rival / Apex / Arctis).
CTRL

cp packaging/debian/gearcraft-daemon.postinst "$DEST/DEBIAN/postinst"
chmod 755 "$DEST/DEBIAN/postinst"

OUT="$ROOT/packaging/${PKG}.deb"
dpkg-deb --build "$DEST" "$OUT"
echo "==> Built $OUT"
ls -lh "$OUT"
