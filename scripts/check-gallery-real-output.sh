#!/usr/bin/env bash
# Fail when a site capture example paints cells after (or instead of) a widget
# rendering. Gallery scenes may only lay out real widgets and hand them data and
# styles through their public API.
#
# Scope: examples/site_widget_gallery.rs, every example named by `--example` in
# scripts/capture-site*.sh, and any `scope` line in scripts/gallery-allowlist.txt.
# Files that do not exist yet are skipped.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

ALLOWLIST="${GALLERY_ALLOWLIST:-scripts/gallery-allowlist.txt}"

files=()
add_file() {
  local path="$1" seen
  for seen in ${files[@]+"${files[@]}"}; do
    [[ "$seen" == "$path" ]] && return 0
  done
  files+=("$path")
}

add_file examples/site_widget_gallery.rs
for script in scripts/capture-site*.sh; do
  [[ -f "$script" ]] || continue
  while IFS= read -r name; do
    add_file "examples/$name.rs"
  done < <(grep -oE -- '--example[ =][A-Za-z0-9_]+' "$script" | sed -E 's/--example[ =]//' | sort -u)
done

helpers=()
if [[ -f "$ALLOWLIST" ]]; then
  lineno=0
  while IFS= read -r line; do
    lineno=$((lineno + 1))
    [[ -z "${line//[[:space:]]/}" || "$line" =~ ^[[:space:]]*# ]] && continue
    if [[ ! "$line" =~ ^(helper|scope)[[:space:]]+([A-Za-z0-9_./-]+)[[:space:]]+#[[:space:]]*[^[:space:]] ]]; then
      echo "$ALLOWLIST:$lineno: expected 'helper NAME # reason' or 'scope PATH # reason'" >&2
      exit 1
    fi
    case "${BASH_REMATCH[1]}" in
      helper) helpers+=("${BASH_REMATCH[2]}") ;;
      scope) add_file "${BASH_REMATCH[2]}" ;;
    esac
  done <"$ALLOWLIST"
fi

allowed=" ${helpers[*]-} "
status=0
checked=0
for file in "${files[@]}"; do
  [[ -f "$file" ]] || continue
  checked=$((checked + 1))
  if ! ALLOWED_HELPERS="$allowed" perl -CSD -0777 -e '
    use strict; use warnings;
    my ($file) = @ARGV;
    open(my $fh, "<:encoding(UTF-8)", $file) or die "$file: $!";
    my $src = do { local $/; <$fh> };
    # Drop comments, keep strings and line numbers.
    $src =~ s{("(?:\\.|[^"\\])*")|//[^\n]*|/\*(.*?)\*/}{
      defined $1 ? $1 : (defined $2 ? ($2 =~ tr/\n//) x "\n" : "")
    }gse;
    my $glyph = qr/[\x{2500}-\x{259F}\x{2800}-\x{28FF}]/;
    my @rules = (
      [qr/\bput_str\b|\bput_cell\b|\bwrite_cell\b|\bset_string\b|\bset_stringn\b|\bset_cell\b|\bset_symbol\b|\bcell_mut\b/,
        "writes cells directly; render a widget instead"],
      [qr/\.(?:put|fill|patch|paint)\s*\(/,
        "looks like painting cells; pass data and styles to a widget instead, or rename the call if it is not a cell write"],
      [qr/\bexact_rows\b/, "hard-coded glyph rows"],
      [qr/widget[-_]targets/, "copies from the widget target mocks"],
      [qr/"[^"\n]*$glyph{4,}[^"\n]*"/, "hard-coded glyph row in a string literal"],
      [qr/"$glyph"\s*\.\s*repeat\s*\(/, "builds a glyph row by hand"],
    );
    my $bad = 0;
    my @lines = split /\n/, $src, -1;
    for my $i (0 .. $#lines) {
      for my $r (@rules) {
        if ($lines[$i] =~ $r->[0]) {
          printf STDERR "%s:%d: %s\n", $file, $i + 1, $r->[1];
          $bad = 1;
        }
      }
    }
    # Helpers that take a render surface can write cells, so each one is named
    # in the allowlist. Widget impls (`render`) are the widgets themselves.
    my $allowed = $ENV{ALLOWED_HELPERS};
    while ($src =~ /\bfn\s+(\w+)\s*(?:<[^>]*>)?\s*\(([^)]*)\)/g) {
      my ($name, $params, $at) = ($1, $2, $-[0]);
      next unless $params =~ /\b(?:RenderCx|Grid|Frame|Buffer)\b/;
      next if $name eq "render" || index($allowed, " $name ") >= 0;
      my $line = (substr($src, 0, $at) =~ tr/\n//) + 1;
      printf STDERR "%s:%d: helper `%s` takes a render surface and is not on the allowlist\n", $file, $line, $name;
      $bad = 1;
    }
    exit($bad);
  ' "$file"; then
    status=1
  fi
done

if [[ "$status" -ne 0 ]]; then
  echo "check-gallery-real-output: demos show real widget output only; no painting over" >&2
  exit 1
fi
echo "check-gallery-real-output: ok ($checked file(s) checked)"
