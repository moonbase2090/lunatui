#!/usr/bin/env bash
# Records the Lunatui and stock Ratatui dashboards as two casts with the same size,
# frame count and scripted ticks. Requires asciinema 3.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
FRAMES="${FRAMES:-1500}"
WIDTH="${WIDTH:-80}"
HEIGHT="${HEIGHT:-24}"
OUT="${OUT:-$ROOT/target/dashboard-casts}"
mkdir -p "$OUT"
cargo build --release -q -p lunatui --example lunatui_dashboard --example ratatui_dashboard
for side in lunatui ratatui; do
  echo "Recording ${side} (${FRAMES} frames, ${WIDTH}x${HEIGHT})..."
  asciinema rec -q --overwrite --window-size "${WIDTH}x${HEIGHT}" -t "${side} dashboard" \
    -c "${ROOT}/target/release/examples/${side}_dashboard --width ${WIDTH} --height ${HEIGHT} --frames ${FRAMES}" \
    "${OUT}/${side}.cast"
done
echo "Casts written to ${OUT}"
