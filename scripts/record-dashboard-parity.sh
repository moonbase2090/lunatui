#!/usr/bin/env bash
# Record paired lunatui vs ratatui dashboard runs with identical scripted ticks.
# Requires: asciinema, cargo, release profile. Side-by-side compositing is manual
# (two players or iTerm splits); both casts use the same --frames value.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
FRAMES="${FRAMES:-1500}"
WIDTH="${WIDTH:-120}"
HEIGHT="${HEIGHT:-36}"
OUT="${ROOT}/examples/captures"
mkdir -p "$OUT"

record() {
  local title="$1"
  local outfile="$2"
  local cmd="$3"
  asciinema rec -q -c "$cmd" -t "$title" "$outfile"
}

LUNA_CMD="cargo run --release -q -p lunatui --example live_dashboard -- --width ${WIDTH} --height ${HEIGHT} --frames ${FRAMES}"
RAT_CMD="cargo run --release -q -p lunatui --example live_dashboard -- --bench --width ${WIDTH} --height ${HEIGHT} --frames ${FRAMES}"

echo "Recording lunatui live dashboard (${FRAMES} frames, ${WIDTH}x${HEIGHT})..."
record "lunatui live_dashboard parity" "$OUT/live_dashboard_parity_lunatui.cast" "$LUNA_CMD"
echo "Recording paired bench transcript (lunatui + ratatui summaries)..."
record "dashboard parity bench" "$OUT/live_dashboard_parity_bench.cast" "$RAT_CMD"
echo "Done. Re-record widget cast: cargo run --release -p lunatui --example gallery -- --demo --frames 900"
echo "Parity numbers: cargo run --release -p lunatui --example live_dashboard -- --parity-report --width 200 --height 60 --frames 240"
