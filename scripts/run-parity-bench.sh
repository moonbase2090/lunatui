#!/usr/bin/env bash
# Same-machine lunatui vs ratatui dashboard parity (identical scripted ticks).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
WIDTH="${WIDTH:-200}"
HEIGHT="${HEIGHT:-60}"
FRAMES="${FRAMES:-240}"
RUNS="${RUNS:-5}"
for i in $(seq 1 "$RUNS"); do
  echo "=== run $i ==="
  cargo run --release -q -p lunatui --example live_dashboard -- \
    --parity-report --width "$WIDTH" --height "$HEIGHT" --frames "$FRAMES"
done
