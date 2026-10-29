#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
EXAMPLE="${EXAMPLE:-kitty_image}"
FEATURES="${FEATURES:-image-presenter,image-decode}"
WIDTH="${WIDTH:-100}"
HEIGHT="${HEIGHT:-32}"
FONT_SIZE="${FONT_SIZE:-20}"
FPS="${FPS:-30}"
SECONDS_TO_RECORD="${SECONDS_TO_RECORD:-8}"
OUT="${OUT:-$ROOT/target/pixel-capture}"
TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target}"
if [[ "$(uname -s)" != Linux ]]; then echo 'record-pixels: requires Linux with Xvfb and Kitty' >&2; exit 2; fi
for tool in Xvfb kitty xdotool ffmpeg python3; do command -v "$tool" >/dev/null || { echo "record-pixels: missing $tool" >&2; exit 2; }; done
for value in "$WIDTH" "$HEIGHT" "$FONT_SIZE" "$FPS" "$SECONDS_TO_RECORD"; do
  if [[ ! "$value" =~ ^[1-9][0-9]{0,3}$ ]]; then echo 'record-pixels: dimensions and timing must be positive integers' >&2; exit 2; fi
done
if [[ ! "$EXAMPLE" =~ ^[a-z][a-z0-9_]*$ ]]; then echo 'record-pixels: invalid example name' >&2; exit 2; fi
mkdir -p "$OUT"
OUT="$(cd "$OUT" && pwd)"
CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-2}" cargo build --release --locked -p lunatui --features "$FEATURES" --example "$EXAMPLE"
X_PID=''
K_PID=''
EVENTS_PID=''
KEYS="${KEYS:-}"
cleanup() {
  if [[ -n "$EVENTS_PID" ]]; then kill "$EVENTS_PID" 2>/dev/null || true; wait "$EVENTS_PID" 2>/dev/null || true; fi
  if [[ -n "$K_PID" ]]; then kill "$K_PID" 2>/dev/null || true; wait "$K_PID" 2>/dev/null || true; fi
  if [[ -n "$X_PID" ]]; then kill "$X_PID" 2>/dev/null || true; wait "$X_PID" 2>/dev/null || true; fi
  rm -f "$OUT/kitty.sock"
}
trap cleanup EXIT INT TERM
Xvfb -displayfd 3 -screen 0 2800x1800x24 -nolisten tcp 3>"$OUT/display.txt" >"$OUT/xvfb.log" 2>&1 &
X_PID=$!
for _ in {1..100}; do [[ -s "$OUT/display.txt" ]] && break; sleep 0.05; done
if [[ ! -s "$OUT/display.txt" ]]; then echo 'record-pixels: Xvfb did not start' >&2; exit 3; fi
export DISPLAY=":$(cat "$OUT/display.txt")"
export LIBGL_ALWAYS_SOFTWARE=1
unset TMUX STY TERM_PROGRAM NO_COLOR
kitty --config NONE --listen-on "unix:$OUT/kitty.sock" --title 'Lunatui Image demo' \
  -o linux_display_server=x11 -o allow_remote_control=yes -o remember_window_size=no \
  -o "initial_window_width=${WIDTH}c" -o "initial_window_height=${HEIGHT}c" \
  -o "font_size=$FONT_SIZE" -o window_padding_width=0 -o hide_window_decorations=yes \
  /bin/sh -c 'sleep 1; exec "$1"' capture "$TARGET_DIR/release/examples/$EXAMPLE" >"$OUT/kitty.log" 2>&1 &
K_PID=$!
WINDOW=''
for _ in {1..100}; do WINDOW="$(xdotool search --onlyvisible --pid "$K_PID" 2>/dev/null | head -1 || true)"; [[ -n "$WINDOW" ]] && break; sleep 0.05; done
if [[ -z "$WINDOW" ]]; then echo 'record-pixels: Kitty window did not appear' >&2; exit 3; fi
xdotool getwindowgeometry --shell "$WINDOW" > "$OUT/geometry.txt"
sleep 2
if [[ -n "$KEYS" ]]; then
  python3 - "$WINDOW" "$KEYS" <<'PYKEYS' &
import subprocess,sys,time
started=time.monotonic()
for event in sys.argv[2].split(','):
    when,key=event.split(':',1)
    delay=float(when)
    if delay < 0 or delay > 60 or not key.isalnum(): raise SystemExit('invalid capture key sequence')
    time.sleep(max(0,started+delay-time.monotonic()))
    subprocess.run(['xdotool','key','--window',sys.argv[1],key],check=True)
PYKEYS
  EVENTS_PID=$!
fi
ffmpeg -nostdin -loglevel error -y -f x11grab -draw_mouse 0 -framerate "$FPS" \
  -window_id "$WINDOW" -i "$DISPLAY" -t "$SECONDS_TO_RECORD" -c:v ffv1 "$OUT/terminal.mkv"
ffmpeg -nostdin -loglevel error -y -sseof -0.1 -i "$OUT/terminal.mkv" -frames:v 1 "$OUT/terminal.png"
ffmpeg -nostdin -loglevel error -y -i "$OUT/terminal.mkv" -c:v libwebp_anim -lossless 1 -loop 0 "$OUT/terminal.webp"
ffprobe -v error -select_streams v:0 -show_entries stream=width,height,r_frame_rate -show_entries format=duration -of json "$OUT/terminal.mkv" > "$OUT/capture.json"
python3 - "$OUT" "$EXAMPLE" "$KEYS" <<'PY'
import hashlib,json,pathlib,subprocess,sys
out=pathlib.Path(sys.argv[1]); receipt=json.loads((out/'capture.json').read_text())
geometry=dict(line.split('=',1) for line in (out/'geometry.txt').read_text().splitlines())
stream=receipt['streams'][0]
assert (stream['width'],stream['height']) == (int(geometry['WIDTH']),int(geometry['HEIGHT'])), 'capture must match the real terminal window'
subprocess.run(['ffmpeg','-nostdin','-loglevel','error','-y','-i',str(out/'terminal.mkv'),'-f','framemd5',str(out/'frames.md5')],check=True)
frames={line.rsplit(',',1)[-1].strip() for line in (out/'frames.md5').read_text().splitlines() if line and not line.startswith('#')}
if sys.argv[3]: assert len(frames)>1, 'scripted input must change the real terminal frames'
assert (out/'terminal.webp').stat().st_size <= 3000000, 'recording exceeds the gallery limit'
receipt['distinct_frames']=len(frames)
receipt['keys']=sys.argv[3]
receipt['example']=sys.argv[2]
receipt['source']='Actual Kitty X11 window captured by ffmpeg x11grab; no compositing.'
receipt['head']=subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip()
receipt['kitty']=subprocess.check_output(['kitty','--version'],text=True).strip().split(' created by ')[0]
receipt['ffmpeg']=subprocess.check_output(['ffmpeg','-version'],text=True).splitlines()[0]
receipt['tracked_worktree_clean']=not subprocess.check_output(['git','status','--porcelain','--untracked-files=no'],text=True).strip()
receipt['sha256']={name:hashlib.sha256((out/name).read_bytes()).hexdigest() for name in ['terminal.mkv','terminal.png','terminal.webp']}
(out/'capture.json').write_text(json.dumps(receipt,indent=2)+'\n')
print('Real terminal recording:',out/'terminal.webp')
PY
