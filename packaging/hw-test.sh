#!/usr/bin/env bash
# GearCraft hardware test helper (run on a machine with real USB peripherals).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

DO_RGB=0
for arg in "$@"; do
  case "$arg" in
    --rgb) DO_RGB=1 ;;
    -h|--help)
      echo "Usage: $0 [--rgb]"
      echo "  Checks deps, udev, lsusb, builds with hidapi, runs GEARCRAFT_SCAN_ONLY."
      echo "  --rgb  also sends a set_color to the first mouse if daemon is already up."
      exit 0
      ;;
  esac
done

echo "==> GearCraft hardware test"
echo "    repo: $ROOT"
echo

ok=0
fail=0
check() {
  local name="$1"; shift
  if "$@"; then
    echo "  [OK] $name"
    ok=$((ok + 1))
  else
    echo "  [!!] $name"
    fail=$((fail + 1))
  fi
}

echo "-- System"
check "cargo available" command -v cargo >/dev/null
check "pkg-config available" command -v pkg-config >/dev/null
if pkg-config --exists hidapi-hidraw 2>/dev/null || pkg-config --exists hidapi 2>/dev/null; then
  echo "  [OK] hidapi pkg-config"
  ok=$((ok + 1))
else
  echo "  [!!] hidapi pkg-config (install libhidapi-dev)"
  fail=$((fail + 1))
fi

echo
echo "-- USB (SteelSeries VID 1038)"
if command -v lsusb >/dev/null; then
  if lsusb -d 1038: 2>/dev/null | grep -q .; then
    lsusb -d 1038: | sed 's/^/  /'
    ok=$((ok + 1))
  else
    echo "  [!!] no USB device with VID 1038 (plug a supported device)"
    fail=$((fail + 1))
  fi
else
  echo "  [!!] lsusb not installed"
  fail=$((fail + 1))
fi

echo
echo "-- udev"
if [[ -f /etc/udev/rules.d/99-gearcraft.rules ]]; then
  echo "  [OK] /etc/udev/rules.d/99-gearcraft.rules present"
  ok=$((ok + 1))
else
  echo "  [!!] udev rules not installed"
  echo "       sudo cp assets/udev/99-gearcraft.rules /etc/udev/rules.d/"
  echo "       sudo udevadm control --reload-rules && sudo udevadm trigger"
  fail=$((fail + 1))
fi

if id -nG 2>/dev/null | grep -qw plugdev; then
  echo "  [OK] user in plugdev"
  ok=$((ok + 1))
else
  echo "  [!!] user not in plugdev (sudo usermod -aG plugdev \$USER && re-login)"
  fail=$((fail + 1))
fi

echo
echo "-- Build + scan (hidapi)"
if cargo build -p gearcraft-daemon --features hidapi 2>&1 | tail -5; then
  echo "  [OK] build with --features hidapi"
  ok=$((ok + 1))
else
  echo "  [!!] build failed"
  fail=$((fail + 1))
fi

echo
echo "-- GEARCRAFT_SCAN_ONLY"
GEARCRAFT_SCAN_ONLY=1 cargo run -q -p gearcraft-daemon --features hidapi || true

if [[ "$DO_RGB" -eq 1 ]]; then
  echo
  echo "-- RGB smoke (requires running daemon)"
  SOCK="${XDG_RUNTIME_DIR:-/tmp}/gearcraft.sock"
  if [[ ! -S "$SOCK" ]]; then
    echo "  [!!] no socket at $SOCK – start daemon first:"
    echo "       cargo run -p gearcraft-daemon --features hidapi"
    fail=$((fail + 1))
  else
    python3 - <<'PY' || true
import json, os, socket, struct, uuid
sock_path = os.environ.get("XDG_RUNTIME_DIR", "/tmp") + "/gearcraft.sock"
req = {"id": str(uuid.uuid4()), "cmd": {"name": "get_devices"}}
body = json.dumps(req).encode()
s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
s.connect(sock_path)
s.sendall(struct.pack("<I", len(body)) + body)
ln = struct.unpack("<I", s.recv(4))[0]
resp = json.loads(s.recv(ln))
s.close()
devs = (resp.get("data") or []) if isinstance(resp.get("data"), list) else []
print("  devices from daemon:", len(devs))
mouse = next((d for d in devs if d.get("kind") == "mouse"), None)
if not mouse:
    print("  [!!] no mouse in daemon list")
    raise SystemExit(0)
print("  testing colour on", mouse.get("name"), mouse.get("id"))
req2 = {
  "id": str(uuid.uuid4()),
  "cmd": {
    "name": "set_color",
    "params": {
      "device_id": mouse["id"],
      "zone": "logo",
      "color": {"r": 0, "g": 231, "b": 160},
    },
  },
}
body = json.dumps(req2).encode()
s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
s.connect(sock_path)
s.sendall(struct.pack("<I", len(body)) + body)
ln = struct.unpack("<I", s.recv(4))[0]
resp = json.loads(s.recv(ln))
s.close()
print("  set_color response:", json.dumps(resp)[:200])
if resp.get("ok"):
    print("  [OK] set_color accepted – check the physical LED")
else:
    print("  [!!] set_color failed")
PY
  fi
fi

echo
echo "==> Summary: $ok ok, $fail issues"
if [[ "$fail" -gt 0 ]]; then
  echo "    See docs/HARDWARE.md for details."
  exit 1
fi
echo "    Hardware path looks ready."
exit 0
