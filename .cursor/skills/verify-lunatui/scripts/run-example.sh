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
run_id=${RUN_ID:-$(date +%s)}
out_dir="$root/.cursor/skills/verify-lunatui/artifacts/$run_id"
mkdir -p "$out_dir"
log="$out_dir/${name}.log"
set +e
if [ $# -gt 0 ]; then
  cargo run -q -p lunatui --example "$name" -- "$@" >"$log" 2>&1
else
  cargo run -q -p lunatui --example "$name" >"$log" 2>&1
fi
status=$?
set -e
echo "wrote $log (exit $status)"
exit "$status"
