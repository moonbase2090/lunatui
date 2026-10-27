#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

CASE="${1:-all}"
OUT="${OUT:-$ROOT/site/assets/media}"
SHEET="${SHEET:-$ROOT/site/proof/widget-contact-sheet.png}"
CASES=(bigtext block calendar gauge lightcycle list overlay paragraph padding scrollbar sparkline spinner statusbar statusdot table tabs textinput borders)
TMP="$(mktemp -d)"
cleanup() { python3 - "$TMP" <<'PY'
import shutil
import sys
shutil.rmtree(sys.argv[1], ignore_errors=True)
PY
}
trap cleanup EXIT

if ! python3 -c 'from fontTools.ttLib import TTFont' >/dev/null 2>&1; then
  echo "Install scripts/requirements-site-capture.txt to convert the bundled JetBrains Mono font." >&2
  exit 2
fi
if ! command -v agg >/dev/null || ! command -v ffmpeg >/dev/null; then
  echo "capture-site-widgets requires agg and ffmpeg." >&2
  exit 2
fi
mkdir -p "$OUT" "$(dirname "$SHEET")" "$TMP/fonts"
PYTHONPATH="${PYTHONPATH:-}" python3 - "$ROOT/site/assets/fonts/JetBrainsMono-Regular.woff2" "$TMP/fonts/JetBrainsMono-Regular.ttf" "$ROOT/site/assets/fonts/JetBrainsMonoNerdFontMono-SemiBold.woff" "$TMP/fonts/JetBrainsMono-Bold.ttf" <<'PY'
import sys
from fontTools.ttLib import TTFont
for source, output in ((sys.argv[1], sys.argv[2]), (sys.argv[3], sys.argv[4])):
    font = TTFont(source)
    font.flavor = None
    if output.endswith("Bold.ttf"):
        for record in font["name"].names:
            if record.nameID in (1, 4, 6, 16, 17):
                values = {1: "JetBrains Mono", 4: "JetBrains Mono Bold", 6: "JetBrainsMono-Bold", 16: "JetBrains Mono", 17: "Bold"}
                record.string = values[record.nameID].encode(record.getEncoding())
        font["OS/2"].usWeightClass = 700
        font["OS/2"].fsSelection = (font["OS/2"].fsSelection & ~1) | 0x20
        font["head"].macStyle |= 1
    font.save(output)
PY

capture() {
  local name="$1"
  cargo run -q -p lunatui --example gallery -- "--site-widget=$name" > "$TMP/$name.ansi"
  python3 - "$TMP/$name.ansi" "$TMP/$name.cast" <<'PY'
import json
import sys
from pathlib import Path
raw = Path(sys.argv[1]).read_text()
header = {"version": 2, "width": 64, "height": 24, "timestamp": 1, "env": {"SHELL": "/bin/sh", "TERM": "xterm-256color"}}
Path(sys.argv[2]).write_text(json.dumps(header) + "\n" + json.dumps([0, "o", raw]) + "\n")
PY
  agg --font-dir "$TMP/fonts" --font-size 30 --line-height 1.2 --text-font-family "JetBrains Mono" \
    "$TMP/$name.cast" "$TMP/$name.gif"
  ffmpeg -loglevel error -y -i "$TMP/$name.gif" -vf 'crop=1152:864:18:18' -frames:v 1 "$OUT/$name.png"
  echo "captured $name"
}

if [[ "$CASE" == all ]]; then
  for name in "${CASES[@]}"; do capture "$name"; done
else
  valid=false
  for name in "${CASES[@]}"; do
    if [[ "$CASE" == "$name" ]]; then valid=true; break; fi
  done
  if [[ "$valid" != true ]]; then
    printf 'unknown widget %q\nvalid cases: %s\n' "$CASE" "${CASES[*]}" >&2
    exit 2
  fi
  capture "$CASE"
fi

ffmpeg -loglevel error -y -pattern_type glob -framerate 1 -i "$OUT/*.png" \
  -vf 'scale=288:216:flags=lanczos,tile=4x5:padding=16:margin=16:color=0x0A0D1C' \
  -frames:v 1 "$SHEET"
echo "contact sheet: $SHEET"
