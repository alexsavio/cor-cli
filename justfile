# cor — JSON log colorizer
# https://github.com/alexsavio/jgb

# Build the project
build:
    cargo build

# Build for release
build-release:
    cargo build --release

# Install cor locally
install:
    cargo install --path .

# Run all tests
test:
    cargo test

# Run tests with output
test-verbose:
    cargo test -- --nocapture

# Run specific test
test-one TEST:
    cargo test {{TEST}} -- --nocapture

# Run benchmarks
bench:
    cargo bench

# Run clippy linter
lint:
    cargo clippy --all-targets --all-features -- -D warnings

# Auto-fix linting issues where possible
lint-fix:
    cargo clippy --all-targets --all-features --fix

# Format code
format:
    cargo fmt

# Check formatting without modifying files
format-check:
    cargo fmt -- --check

# Run all quality checks
check: format-check lint test

# Generate documentation
doc:
    cargo doc --no-deps --open

# Clean build artifacts
clean:
    cargo clean

# Full clean and rebuild
rebuild: clean build

# Run the demo (colorized)
demo:
    #!/usr/bin/env bash
    set -euo pipefail
    BIN="cargo run -q --"

    echo "━━━ Default colorized output ━━━"
    echo "$ cat assets/demo.jsonl | cor"
    echo ""
    cat assets/demo.jsonl | $BIN --color=always
    echo ""

    echo "━━━ Filter: warn and above ━━━"
    echo "$ cat assets/demo.jsonl | cor --level warn"
    echo ""
    cat assets/demo.jsonl | $BIN --color=always --level warn
    echo ""

    echo "━━━ Include only specific fields ━━━"
    echo "$ cat assets/demo.jsonl | cor -i method,path,status"
    echo ""
    cat assets/demo.jsonl | $BIN --color=always -i method,path,status
    echo ""

    echo "━━━ Exclude fields ━━━"
    echo "$ cat assets/demo.jsonl | cor -e func,query"
    echo ""
    cat assets/demo.jsonl | $BIN --color=always -e func,query
    echo ""

    echo "━━━ JSON output (filtered) ━━━"
    echo "$ cat assets/demo.jsonl | cor --json --level error"
    echo ""
    cat assets/demo.jsonl | $BIN --json --level error
    echo ""

    echo "━━━ Truncate long field values ━━━"
    echo "$ cat assets/demo.jsonl | cor --max-field-length 20"
    echo ""
    cat assets/demo.jsonl | $BIN --color=always --max-field-length 20

# Generate coverage report
coverage:
    cargo tarpaulin --out Html --output-dir coverage

# Security audit
audit:
    cargo audit

# Install development tools
dev-tools:
    cargo install cargo-watch
    cargo install cargo-tarpaulin
    cargo install cargo-audit
    cargo install git-cliff

# Publish to crates.io (dry-run first)
publish-dry:
    cargo publish --dry-run

# Publish to crates.io
publish: check
    cargo publish

# =============================================================================
# Release Management
# =============================================================================

# Show current version
version:
    @grep '^version' Cargo.toml | head -1 | cut -d'"' -f2

# Generate/update CHANGELOG.md
changelog:
    git-cliff -o CHANGELOG.md

# Preview changelog for next release (unreleased changes)
changelog-preview:
    git-cliff --unreleased --strip header

# Compute next CalVer version (YYYY.MM.MICRO)
_next-version:
    #!/usr/bin/env bash
    set -euo pipefail
    YEAR=$(date +%Y)
    MONTH=$(date +%-m)
    PREFIX="${YEAR}.${MONTH}"
    CURRENT=$(grep '^version' Cargo.toml | head -1 | cut -d'"' -f2)
    if [[ "$CURRENT" == ${PREFIX}.* ]]; then
        MICRO=${CURRENT##*.}
        echo "${PREFIX}.$((MICRO + 1))"
    else
        echo "${PREFIX}.0"
    fi

# Create a new release with explicit version
# Usage: just release 2026.2.1
release VERSION:
    #!/usr/bin/env bash
    set -euo pipefail

    echo "📦 Releasing v{{VERSION}} (CalVer)"

    # Update Cargo.toml version
    sed -i '' 's/^version = ".*"/version = "{{VERSION}}"/' Cargo.toml

    # Ensure it compiles and passes checks
    just check

    # Update CHANGELOG.md
    git-cliff --tag "v{{VERSION}}" -o CHANGELOG.md

    # Update Cargo.lock
    cargo check

    # Commit, tag, and push
    git add Cargo.toml Cargo.lock CHANGELOG.md
    git commit -m "chore: release v{{VERSION}}"
    git tag "v{{VERSION}}"
    git push
    git push origin "v{{VERSION}}"

    echo "✅ Released v{{VERSION}}"
    echo "   Run 'just publish' to push to crates.io"

# Create a new release with auto-computed CalVer version
release-next:
    #!/usr/bin/env bash
    set -euo pipefail
    VERSION=$(just _next-version)
    just release "$VERSION"

# =============================================================================
# Help
# =============================================================================

# Show help
help:
    @just --list
