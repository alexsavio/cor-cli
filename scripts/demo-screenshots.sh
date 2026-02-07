#!/usr/bin/env bash
# Generate PNG screenshots for each part of the cor demo.
# Requires: termshot (brew install homeport/tap/termshot)
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_DIR"

OUT_DIR="$PROJECT_DIR/assets/demo"
mkdir -p "$OUT_DIR"

cargo build -q --release
BIN="$PROJECT_DIR/target/release/cor"

# Create a temporary directory with a 'cor' symlink so termshot shows 'cor' in --show-cmd
TMPBIN="$(mktemp -d)"
ln -sf "$BIN" "$TMPBIN/cor"
export PATH="$TMPBIN:$PATH"

trap 'rm -rf "$TMPBIN"' EXIT

echo "Generating demo screenshots in $OUT_DIR ..."

# Helper: capture a command's output as a screenshot
# Usage: shot <output.png> <display_cmd> [actual_cmd]
# If actual_cmd is omitted, display_cmd is used for execution too.
shot() {
  local file="$1"
  local display_cmd="$2"
  local actual_cmd="${3:-$2}"
  local tmpraw
  tmpraw="$(mktemp)"

  echo "  $ $display_cmd → $(basename "$file")"

  # Pre-render the prompt + command output to a temp file
  printf '❯ %s\n' "$display_cmd" > "$tmpraw"
  bash -c "$actual_cmd" >> "$tmpraw" 2>&1

  # Feed the captured ANSI output to termshot (no PTY needed)
  termshot --filename "$file" --raw-read "$tmpraw"
  rm -f "$tmpraw"
}

# 1 — Default colorized output
shot "$OUT_DIR/01-default.png" \
  "cat assets/demo.jsonl | cor" \
  "cat assets/demo.jsonl | cor -c always"

# 2 — Filter: warn and above
shot "$OUT_DIR/02-level-filter.png" \
  "cat assets/demo.jsonl | cor --level warn" \
  "cat assets/demo.jsonl | cor -c always --level warn"

# 3 — Include only specific fields
shot "$OUT_DIR/03-include-fields.png" \
  "cat assets/demo.jsonl | cor -i method,path,status" \
  "cat assets/demo.jsonl | cor -c always -i method,path,status"

# 4 — Exclude fields
shot "$OUT_DIR/04-exclude-fields.png" \
  "cat assets/demo.jsonl | cor -e func,query" \
  "cat assets/demo.jsonl | cor -c always -e func,query"

# 5 — JSON output (filtered)
shot "$OUT_DIR/05-json-output.png" \
  "cat assets/demo.jsonl | cor --json --level error | grep -v '^$'" \
  "cat assets/demo.jsonl | cor -c always --json --level error | grep -v '^$'"

# 6 — Truncate long field values
shot "$OUT_DIR/06-truncate-fields.png" \
  "cat assets/demo.jsonl | cor --max-field-length 20" \
  "cat assets/demo.jsonl | cor -c always --max-field-length 20"

echo ""
echo "Done! Screenshots saved to $OUT_DIR:"
ls -1 "$OUT_DIR"
