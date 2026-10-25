#!/usr/bin/env bash
# Side-by-side E2E dashboard parity bench (same Model, size, frames, machine).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
OUT="${1:-examples/captures/parity}"
mkdir -p "$OUT"
cargo build --release -p lunatui --example dashboard_parity_bench
{
  echo "# dashboard parity bench"
  echo "# host: $(uname -srm)"
  echo "# date: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo
  /usr/bin/time -l cargo run -q --release -p lunatui --example dashboard_parity_bench -- --width 80 --height 24 --frames 240 --runs 5
  echo
  /usr/bin/time -l cargo run -q --release -p lunatui --example dashboard_parity_bench -- --width 200 --height 60 --frames 240 --runs 5
} 2>&1 | tee "$OUT/dashboard_parity_bench_full.txt"
