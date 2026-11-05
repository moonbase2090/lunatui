#!/usr/bin/env bash
# Records the effects comparison (native Effect fade beside a hand-written
# ratatui fade) through the widget recording path, at both sizes, both themes
# and three color profiles.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="${OUT:-$ROOT/target/effects-recordings}"
for size in 80x24 200x60; do
  width="${size%x*}"
  height="${size#*x}"
  for theme in dark light; do
    for profile in truecolor ansi256 ansi16; do
      CASE=effects WIDTH="$width" HEIGHT="$height" THEME="$theme" PROFILE="$profile" \
        FRAMES="${FRAMES:-240}" FPS="${FPS:-30}" OUT="$OUT" \
        bash "$ROOT/scripts/record-widget.sh"
    done
  done
done
