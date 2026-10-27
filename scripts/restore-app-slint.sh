#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DIR="$ROOT/crates/gearcraft-ui/ui"
cat "$DIR/app.slint.gz.b64.part1a" "$DIR/app.slint.gz.b64.part1b" \
    "$DIR/app.slint.gz.b64.part2a" "$DIR/app.slint.gz.b64.part2b" \
  | base64 -d | gzip -dc > "$DIR/app.slint"
echo "Restored $DIR/app.slint ($(wc -c < "$DIR/app.slint") bytes)"
