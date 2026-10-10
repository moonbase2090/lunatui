#!/usr/bin/env bash
# Scan release-facing text and built artifacts for disallowed patterns.
# Prints only path:line (or path:strings:line) on failure; never prints denylist terms.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

ALLOWED_EMAIL='322824348+mb2090@users.noreply.github.com'
DENYLIST_FILE="${RELEASE_DENYLIST_FILE:-$HOME/.config/moonbase/release-denylist.txt}"

EXTRA_TEXT_FILES=()
ARTIFACT_DIRS=()
TAG_MESSAGE=""
RELEASE_TITLE=""
RELEASE_BODY=""

usage() {
  echo "usage: $0 [--tag-message TEXT] [--release-title TEXT] [--release-body TEXT] [--artifact-dir DIR] [--file PATH]..." >&2
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --tag-message)
      TAG_MESSAGE="${2:-}"
      shift 2
      ;;
    --release-title)
      RELEASE_TITLE="${2:-}"
      shift 2
      ;;
    --release-body)
      RELEASE_BODY="${2:-}"
      shift 2
      ;;
    --artifact-dir)
      ARTIFACT_DIRS+=("${2:-}")
      shift 2
      ;;
    --file)
      EXTRA_TEXT_FILES+=("${2:-}")
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "unknown argument: $1" >&2
      usage
      exit 2
      ;;
  esac
done

GENERIC_PATTERNS=(
  'DESKTOP-[A-Z0-9]+'
  '\.local\b'
  '/Users/'
  '/home/[a-z]'
  'C:\\Users'
  '-----BEGIN [A-Z ]*PRIVATE KEY-----'
  '(ghp|gho|ghs|ghu|github_pat)_[A-Za-z0-9_]{20,}'
  '\b(10\.[0-9]+\.[0-9]+\.[0-9]+|192\.168\.[0-9]+\.[0-9]+|172\.(1[6-9]|2[0-9]|3[01])\.[0-9]+\.[0-9]+)\b'
)

EMAIL_PATTERN='[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}'

TMP_EXTRA="$(mktemp)"
TMP_GENERIC="$(mktemp)"
TMP_FULL="$(mktemp)"
trap 'rm -f "$TMP_EXTRA" "$TMP_EXTRA.tag" "$TMP_EXTRA.title" "$TMP_EXTRA.body" "$TMP_GENERIC" "$TMP_FULL"' EXIT

RELEASE_SURFACE_FILES=()
if [[ -n "$TAG_MESSAGE" ]]; then
  printf '%s\n' "$TAG_MESSAGE" >"$TMP_EXTRA.tag"
  RELEASE_SURFACE_FILES+=("$TMP_EXTRA.tag")
fi
if [[ -n "$RELEASE_TITLE" ]]; then
  printf '%s\n' "$RELEASE_TITLE" >"$TMP_EXTRA.title"
  RELEASE_SURFACE_FILES+=("$TMP_EXTRA.title")
fi
if [[ -n "$RELEASE_BODY" ]]; then
  printf '%s\n' "$RELEASE_BODY" >"$TMP_EXTRA.body"
  RELEASE_SURFACE_FILES+=("$TMP_EXTRA.body")
fi

write_patterns() {
  local dest="$1"
  local include_denylist="$2"
  {
    for pat in "${GENERIC_PATTERNS[@]}"; do
      printf '%s\n' "$pat"
    done
    if [[ "$include_denylist" == "1" ]]; then
      if [[ -f "$DENYLIST_FILE" ]]; then
        grep -v '^[[:space:]]*$' "$DENYLIST_FILE" | grep -v '^[[:space:]]*#' || true
      fi
      if [[ -n "${RELEASE_DENYLIST:-}" ]]; then
        printf '%s\n' "$RELEASE_DENYLIST"
      fi
    fi
  } >"$dest"
}

write_patterns "$TMP_GENERIC" 0
write_patterns "$TMP_FULL" 1

collect_changelog_files() {
  local f
  for f in CHANGELOG CHANGELOG.md RELEASING.md; do
    [[ -f "$f" ]] && printf '%s\n' "$f"
  done
  while IFS= read -r -d '' f; do
    printf '%s\n' "$f"
  done < <(find . -maxdepth 2 -type f \( -iname 'release-notes*' -o -iname 'RELEASE_NOTES*' \) ! -path './.git/*' -print0 2>/dev/null || true)
}

collect_doc_files() {
  [[ -f README.md ]] && printf '%s\n' README.md
  while IFS= read -r -d '' f; do
    printf '%s\n' "$f"
  done < <(find docs -type f \( -name '*.md' -o -name '*.txt' \) ! -path './.git/*' -print0 2>/dev/null || true)
}

report_text_hit() {
  local file="$1"
  local line_no="$2"
  echo "${file}:${line_no}"
}

scan_text_file() {
  local file="$1"
  local patterns="$2"
  local pat line_no
  while IFS= read -r pat || [[ -n "$pat" ]]; do
    [[ -z "$pat" ]] && continue
    while IFS= read -r line_no; do
      [[ -n "$line_no" ]] && report_text_hit "$file" "$line_no"
    done < <(grep -niE "$pat" "$file" 2>/dev/null | cut -d: -f1 || true)
  done <"$patterns"
}

check_disallowed_emails() {
  local file="$1"
  local line_no content email
  while IFS= read -r line_no; do
    [[ -z "$line_no" ]] && continue
    content="$(sed -n "${line_no}p" "$file")"
    while IFS= read -r email; do
      [[ -z "$email" ]] && continue
      if [[ "$email" != "$ALLOWED_EMAIL" ]]; then
        report_text_hit "$file" "$line_no"
      fi
    done < <(grep -oE "$EMAIL_PATTERN" <<<"$content" || true)
  done < <(grep -niE "$EMAIL_PATTERN" "$file" 2>/dev/null | cut -d: -f1 || true)
}

scan_text_file_full() {
  local file="$1"
  scan_text_file "$file" "$TMP_FULL"
  check_disallowed_emails "$file"
}

scan_text_file_generic() {
  local file="$1"
  scan_text_file "$file" "$TMP_GENERIC"
  check_disallowed_emails "$file"
}

scan_strings_artifact() {
  local artifact="$1"
  local pat line_no
  while IFS= read -r pat || [[ -n "$pat" ]]; do
    [[ -z "$pat" ]] && continue
    while IFS= read -r line_no; do
      [[ -n "$line_no" ]] && echo "${artifact}:strings:${line_no}"
    done < <(strings "$artifact" 2>/dev/null | grep -niE "$pat" 2>/dev/null | cut -d: -f1 || true)
  done <"$TMP_FULL"

  line_no=0
  while IFS= read -r content; do
    line_no=$((line_no + 1))
    while IFS= read -r email; do
      [[ -z "$email" ]] && continue
      if [[ "$email" != "$ALLOWED_EMAIL" ]]; then
        echo "${artifact}:strings:${line_no}"
      fi
    done < <(grep -oE "$EMAIL_PATTERN" <<<"$content" || true)
  done < <(strings "$artifact" 2>/dev/null || true)
}

record_hits() {
  local hit
  while IFS= read -r hit; do
    [[ -z "$hit" ]] && continue
    echo "$hit"
    HITS=1
  done
}

HITS=0

while IFS= read -r file; do
  [[ -z "$file" ]] && continue
  record_hits < <(scan_text_file_generic "$file")
done < <(collect_doc_files | sort -u)

while IFS= read -r file; do
  [[ -z "$file" ]] && continue
  record_hits < <(scan_text_file_full "$file")
done < <(collect_changelog_files | sort -u)

for file in "${RELEASE_SURFACE_FILES[@]}"; do
  [[ -f "$file" ]] || continue
  record_hits < <(scan_text_file_full "$file")
done

for file in "${EXTRA_TEXT_FILES[@]}"; do
  [[ -f "$file" ]] || continue
  record_hits < <(scan_text_file_full "$file")
done

DEFAULT_ARTIFACT_DIR="$ROOT/target/release/examples"
if [[ ${#ARTIFACT_DIRS[@]} -eq 0 && -d "$DEFAULT_ARTIFACT_DIR" ]]; then
  ARTIFACT_DIRS+=("$DEFAULT_ARTIFACT_DIR")
fi

for dir in "${ARTIFACT_DIRS[@]}"; do
  [[ -d "$dir" ]] || continue
  while IFS= read -r -d '' artifact; do
    [[ -f "$artifact" ]] || continue
    case "$artifact" in
      *.rlib | *.a | *.d | *.rmeta) continue ;;
    esac
    base="${artifact##*/}"
    if [[ "$base" =~ \.(tar|tar\.gz|tgz|zip|dmg|msi|deb)$ ]]; then
      record_hits < <(scan_strings_artifact "$artifact")
      continue
    fi
    if [[ -x "$artifact" ]] || file "$artifact" 2>/dev/null | grep -q 'executable'; then
      record_hits < <(scan_strings_artifact "$artifact")
    fi
  done < <(find "$dir" -maxdepth 1 -type f -print0 2>/dev/null || true)
done

if [[ "$HITS" -ne 0 ]]; then
  echo "check-release-text: disallowed release text detected (see path:line above)" >&2
  exit 1
fi

echo "check-release-text: ok"
