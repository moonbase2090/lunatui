#!/usr/bin/env sh
set -eu
if [ $# -lt 1 ]; then
  echo "usage: run-example.sh <example-name> [-- cargo args...]" >&2
  exit 2
fi
name=$1
shift
if [ "${1:-}" = "--" ]; then
  shift
fi
root=$(cd "$(dirname "$0")/../../../.." && pwd)
cd "$root"

safe_component() {
  case "$1" in
    ''|*/*|*..*|.[!.]*|..) return 1 ;;
  esac
  return 0
}

refuse_bad_log_dest() {
  if [ -L "$log" ]; then
    echo "log path is a symlink; refusing to write" >&2
    return 1
  fi
  if [ -d "$log" ]; then
    echo "log path is a directory; refusing to write" >&2
    return 1
  fi
  if [ -e "$log" ] && [ ! -f "$log" ]; then
    echo "log path must be absent or a regular file" >&2
    return 1
  fi
  return 0
}

run_id=${RUN_ID:-$(date +%s)}
if ! safe_component "$run_id" || ! safe_component "$name"; then
  echo "RUN_ID and example name must be single safe path components" >&2
  exit 2
fi

artifacts_root="$root/.cursor/skills/verify-lunatui/artifacts"
mkdir -p "$artifacts_root"
out_dir="$artifacts_root/$run_id"
mkdir -p "$out_dir"
artifacts_root=$(cd "$artifacts_root" && pwd -P)
out_dir=$(cd "$out_dir" && pwd -P)
case "$out_dir" in
  "$artifacts_root"/*) ;;
  *)
    echo "artifact directory escapes $artifacts_root" >&2
    exit 2
    ;;
esac
log="$out_dir/${name}.log"
case "$log" in
  "$out_dir"/*) ;;
  *)
    echo "log path escapes artifact directory" >&2
    exit 2
    ;;
esac

if ! refuse_bad_log_dest; then
  exit 2
fi

tmp="$out_dir/.${name}.log.$$"
umask 077
if [ -e "$tmp" ] || [ -L "$tmp" ]; then
  echo "temporary log path already exists" >&2
  exit 2
fi
: >"$tmp"

set +e
if [ $# -gt 0 ]; then
  cargo run -q -p lunatui --example "$name" -- "$@" >"$tmp" 2>&1
else
  cargo run -q -p lunatui --example "$name" >"$tmp" 2>&1
fi
status=$?
set -e

if ! refuse_bad_log_dest; then
  rm -f "$tmp"
  exit 2
fi
mv -f "$tmp" "$log"
if [ ! -f "$log" ]; then
  echo "failed to install log at $log" >&2
  exit 2
fi
echo "wrote $log (exit $status)"
exit "$status"
