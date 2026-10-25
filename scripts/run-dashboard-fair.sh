#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="${DASHBOARD_FAIR_OUT:-$ROOT/target/dashboard-fair}"
FRAMES="${FRAMES:-240}"
RUNS="${RUNS:-5}"
mkdir -p "$OUT"
cd "$ROOT"
cargo build --release -p lunatui \
  --example dashboard_fair_lunatui \
  --example dashboard_fair_ratatui \
  --example dashboard_fair_report \
  --example lunatui_dashboard \
  --example ratatui_dashboard
for size in "80 24" "200 60"; do
  set -- $size
  w=$1
  h=$2
  log="$OUT/dashboard_fair_${w}x${h}.log"
  echo "=== dashboard_fair ${w}x${h} ===" | tee "$log"
  cargo run -q --release -p lunatui --example dashboard_fair_report -- \
    --width "$w" --height "$h" --frames "$FRAMES" --runs "$RUNS" 2>&1 | tee -a "$log"
done
