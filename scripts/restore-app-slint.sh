#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
B64="$ROOT/crates/gearcraft-ui/ui/app.slint.gz.b64"
OUT="$ROOT/crates/gearcraft-ui/ui/app.slint"
base64 -d < "$B64" | gzip -dc > "$OUT"
echo "Restored $OUT ($(wc -c < "$OUT") bytes)"
