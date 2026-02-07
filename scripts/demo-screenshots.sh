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
shot() {
  local file="$1"; shift
  local cmd="$*"
  echo "  $ $cmd → $(basename "$file")"
  termshot --filename "$file" -- \
    bash -c "printf '❯ %s\n' '$cmd' && $cmd"
}

# 1 — Default colorized output
shot "$OUT_DIR/01-default.png" \
  "cat assets/demo.jsonl | cor"

# 2 — Filter: warn and above
shot "$OUT_DIR/02-level-filter.png" \
  "cat assets/demo.jsonl | cor --level warn"

# 3 — Include only specific fields
shot "$OUT_DIR/03-include-fields.png" \
  "cat assets/demo.jsonl | cor -i method,path,status"

# 4 — Exclude fields
shot "$OUT_DIR/04-exclude-fields.png" \
  "cat assets/demo.jsonl | cor -e func,query"

# 5 — JSON output (filtered)
shot "$OUT_DIR/05-json-output.png" \
  "cat assets/demo.jsonl | cor --json --level error | grep -v '^$'"

# 6 — Truncate long field values
shot "$OUT_DIR/06-truncate-fields.png" \
  "cat assets/demo.jsonl | cor --max-field-length 20"

echo ""
echo "Done! Screenshots saved to $OUT_DIR:"
ls -1 "$OUT_DIR"
