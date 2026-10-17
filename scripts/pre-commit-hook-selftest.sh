#!/usr/bin/env bash
# Regression: staged blob must be checked; formatted worktree cannot mask unformatted index.
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
tmpdir="$(mktemp -d)"
trap 'rm -rf "$tmpdir"' EXIT

cd "$tmpdir"
git init -q
git config user.email '322824348+mb2090@users.noreply.github.com'
git config user.name 'mb2090'
git config core.hooksPath "$root/.githooks"

cat > Cargo.toml <<'EOF'
[package]
name = "hook_selftest"
version = "0.0.0"
edition = "2021"
publish = false
EOF

mkdir src
printf 'fn main(){let x=1;}\n' > src/main.rs
git add Cargo.toml src/main.rs
git commit -q -m init

printf 'fn main(){let x=1;}\n' > src/main.rs
git add src/main.rs
printf 'fn main(){\nlet x=1;\n}\n' > src/main.rs

if git commit -m should-fail -q 2>/dev/null; then
  printf 'expected pre-commit to reject partial staging\n' >&2
  exit 1
fi

printf 'fn main(){\nlet x=1;\n}\n' > src/main.rs
git add src/main.rs
if git commit -m should-fail-fmt -q 2>/dev/null; then
  printf 'expected pre-commit to reject unformatted staged rust\n' >&2
  exit 1
fi

cargo fmt --all
git add src/main.rs
git commit -q -m ok

printf 'pre-commit-hook-selftest: ok\n'
