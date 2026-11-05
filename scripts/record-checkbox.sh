#!/usr/bin/env bash
# Record the actual Checkbox implementations through the existing widget speed-test path.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="${OUT:-$ROOT/target/checkbox-recordings}"
for size in 80x24 200x60; do
  width="${size%x*}"
  height="${size#*x}"
  for theme in dark light; do
    for profile in truecolor ansi256 ansi16; do
      CASE=checkbox WIDTH="$width" HEIGHT="$height" THEME="$theme" PROFILE="$profile" \
        FRAMES="${FRAMES:-240}" FPS="${FPS:-30}" OUT="$OUT" \
        bash "$ROOT/scripts/record-widget.sh"
    done
  done
done
