#!/usr/bin/env bash
# Regression checks for scripts/trim-cast.py: the leading blank and the alternate-screen exit
# are dropped, and with a frame rate each frame lands on its fixed-timestep slot.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

# Three frames recorded with jitter (31 ms and 37 ms apart), the second split into two
# writes 2 ms apart, then the exit from the alternate screen.
write_cast() {
  cat >"$1" <<'CAST'
{"version": 3, "term": {"cols": 4, "rows": 1}}
[0.5, "o", "a"]
[0.031, "o", "b"]
[0.002, "o", "c"]
[0.037, "o", "d"]
[0.033, "o", "\u001b[?1049l"]
CAST
}

check() {
  local label="$1" want="$2" got
  got="$(python3 -I - "$3" <<'PY'
import json, sys
lines = open(sys.argv[1], encoding="utf-8").read().splitlines()[1:]
print(" ".join(f"{json.loads(line)[0]:.6f}:{json.loads(line)[2]}" for line in lines))
PY
)"
  if [[ "$got" != "$want" ]]; then
    echo "trim-cast-selftest: $label: expected '$want', got '$got'" >&2
    exit 1
  fi
}

write_cast "$tmp/plain.cast"
python3 -I "$ROOT/scripts/trim-cast.py" "$tmp/plain.cast"
check "trim only" "0.000000:a 0.031000:b 0.002000:c 0.037000:d" "$tmp/plain.cast"

# Frame starts move to 1/30 s and 2/30 s; the split write keeps its 2 ms offset, so the
# third frame follows it by 2/30 - 1/30 - 0.002 s.
write_cast "$tmp/snap.cast"
python3 -I "$ROOT/scripts/trim-cast.py" "$tmp/snap.cast" 30
check "snapped to 30 fps" "0.000000:a 0.033333:b 0.002000:c 0.031333:d" "$tmp/snap.cast"

echo "trim-cast-selftest: ok"
