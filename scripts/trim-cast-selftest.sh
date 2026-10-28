#!/usr/bin/env bash
# Regression checks for scripts/trim-cast.py: the leading blank and the alternate-screen exit
# are dropped, and with a frame rate each frame becomes one event on its fixed-timestep slot.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

# Without frame marks: three frames recorded with jitter (31 ms and 37 ms apart), the second
# split into two writes 2 ms apart, then the exit from the alternate screen.
write_plain() {
  cat >"$1" <<'CAST'
{"version": 3, "term": {"cols": 4, "rows": 1}}
[0.5, "o", "a"]
[0.031, "o", "b"]
[0.002, "o", "c"]
[0.037, "o", "d"]
[0.033, "o", "\u001b[?1049l"]
CAST
}

# With frame marks: the recorder merged the end of frame A with the start of frame C into one
# write, split C's close into its own write 4 ms later, and stamped D only 5 ms after that.
write_marked() {
  cat >"$1" <<'CAST'
{"version": 3, "term": {"cols": 4, "rows": 1}}
[0.5, "o", "\u001b[?1049h\u001b[?2026hA"]
[0.012, "o", "B\u001b[?2026l\u001b[?2026hC"]
[0.004, "o", "\u001b[?2026l"]
[0.005, "o", "\u001b[?2026hD\u001b[?2026l"]
[0.02, "o", "\u001b[?1049l"]
CAST
}

check() {
  local label="$1" want="$2" got
  got="$(python3 -I - "$3" <<'PY'
import json, re, sys
lines = open(sys.argv[1], encoding="utf-8").read().splitlines()[1:]
events = [json.loads(line) for line in lines]
modes = re.compile(r"\x1b\[\?\d+[hl]")
print(" ".join(f"{e[0]:.6f}:{modes.sub('', e[2])}" for e in events))
PY
)"
  if [[ "$got" != "$want" ]]; then
    echo "trim-cast-selftest: $label: expected '$want', got '$got'" >&2
    exit 1
  fi
}

write_plain "$tmp/plain.cast"
python3 -I "$ROOT/scripts/trim-cast.py" "$tmp/plain.cast"
check "trim only" "0.000000:a 0.031000:b 0.002000:c 0.037000:d" "$tmp/plain.cast"

# Frames split at pauses land at 0, 1/30 s, and 2/30 s; the split write joins its frame.
write_plain "$tmp/snap.cast"
python3 -I "$ROOT/scripts/trim-cast.py" "$tmp/snap.cast" 30
check "snapped at pauses" "0.000000:a 0.033333:bc 0.033333:d" "$tmp/snap.cast"

# Frames split at the marks, whatever the recorder did with the writes.
write_marked "$tmp/marked.cast"
python3 -I "$ROOT/scripts/trim-cast.py" "$tmp/marked.cast" 60 3
check "snapped at marks" "0.000000:AB 0.016667:C 0.016667:D" "$tmp/marked.cast"

write_marked "$tmp/short.cast"
if python3 -I "$ROOT/scripts/trim-cast.py" "$tmp/short.cast" 60 4 2>/dev/null; then
  echo "trim-cast-selftest: a recording with 3 of 4 frames was accepted" >&2
  exit 1
fi

# gif-to-webp: slots on a 30 fps timeline shown at 60 fps take 17 and 16 ms in turn, a frame
# agg merged with its identical neighbour keeps both slots, and the last frame is held.
got="$(python3 -I - "$ROOT/scripts/gif-to-webp.py" <<'PY'
import importlib.util, sys
sys.dont_write_bytecode = True
spec = importlib.util.spec_from_file_location("gif_to_webp", sys.argv[1])
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)
print(module.durations([0.0, 0.03, 0.07, 0.13], 60, 2000, 30))
PY
)"
if [[ "$got" != "[17, 16, 34, 2000]" ]]; then
  echo "trim-cast-selftest: gif-to-webp durations: expected '[17, 16, 34, 2000]', got '$got'" >&2
  exit 1
fi

echo "trim-cast-selftest: ok"
