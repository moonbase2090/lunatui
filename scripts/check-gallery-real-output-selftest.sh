#!/usr/bin/env bash
# Regression checks for scripts/check-gallery-real-output.sh.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

mkdir -p "$tmp/scripts" "$tmp/examples"
cp "$ROOT/scripts/check-gallery-real-output.sh" "$tmp/scripts/"
printf '# empty\n' >"$tmp/scripts/gallery-allowlist.txt"
check="$tmp/scripts/check-gallery-real-output.sh"

expect() {
  local want="$1" label="$2" rc=0
  "$check" >/dev/null 2>&1 || rc=$?
  if [[ "$want" == pass && "$rc" -ne 0 ]] || [[ "$want" == fail && "$rc" -eq 0 ]]; then
    echo "check-gallery-real-output-selftest: $label: expected $want, got exit $rc" >&2
    exit 1
  fi
}

gallery="$tmp/examples/site_widget_gallery.rs"

printf 'fn main() { Block::new().render(area, cx); }\n' >"$gallery"
expect pass "widget calls only"

printf 'fn main() { cx.put_str(1, 1, "x", s); }\n' >"$gallery"
expect fail "put_str"
printf 'fn main() { grid.set_string(1, 1, "x", s); }\n' >"$gallery"
expect fail "set_string"
printf 'fn main() { grid.cell_mut(1, 1); }\n' >"$gallery"
expect fail "cell_mut"
printf 'fn main() { painter.fill(rect); }\n' >"$gallery"
expect fail "fill"
printf 'const R: &[&str] = &["xx"]; fn main() { exact_rows(R); }\n' >"$gallery"
expect fail "exact_rows"
printf 'const T: &str = "▁▂▃▄▅▆▇█";\n' >"$gallery"
expect fail "glyph row literal"
printf 'fn main() { let t = "─".repeat(60); }\n' >"$gallery"
expect fail "glyph repeat"
printf '// widget-targets mock\nfn main() { /* put_str */ }\n' >"$gallery"
expect pass "comments are ignored"
printf 'fn main() { let u = "http://x"; cx.put_str(1, 1, u, s); }\n' >"$gallery"
expect fail "string with slashes does not hide a call"

printf 'fn row(cx: &mut RenderCx<'"'"'_>, y: u16) {}\n' >"$gallery"
expect fail "unlisted helper"
printf 'helper row # lays out a Block row\n' >"$tmp/scripts/gallery-allowlist.txt"
expect pass "listed helper"
printf 'helper row\n' >"$tmp/scripts/gallery-allowlist.txt"
expect fail "allowlist entry without a reason"

echo "check-gallery-real-output-selftest: ok"
