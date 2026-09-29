# cor — JSON log colorizer
# https://github.com/alexsavio/cor-cli

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
    rumdl fmt *.md

# Check formatting without modifying files
format-check:
    cargo fmt -- --check
    rumdl check *.md

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

# Generate demo screenshots (requires termshot)
screenshots:
    scripts/demo-screenshots.sh

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
    cargo install rumdl

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
    @sed -n '/^\[package\]/,/^\[/{s/^version = "\(.*\)"/\1/p;}' Cargo.toml

# Generate/update CHANGELOG.md
changelog:
    git-cliff -o CHANGELOG.md

# Preview changelog for next release (unreleased changes)
changelog-preview:
    git-cliff --unreleased --strip header -o -

# Compute next CalVer version (YYYY.MM.MICRO)
_next-version:
    #!/usr/bin/env bash
    set -euo pipefail
    YEAR=$(date +%Y)
    MONTH=$(date +%-m)
    PREFIX="${YEAR}.${MONTH}"
    CURRENT=$(sed -n '/^\[package\]/,/^\[/{s/^version = "\(.*\)"/\1/p;}' Cargo.toml)
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

    if [[ ! "{{VERSION}}" =~ ^[0-9]{4}\.[1-9][0-9]?\.[0-9]+$ ]]; then
        echo "VERSION must be YYYY.MM.MICRO, e.g. 2026.9.0 (no leading v)" >&2
        exit 1
    fi
    if git rev-parse -q --verify "refs/tags/v{{VERSION}}" >/dev/null; then
        echo "tag v{{VERSION}} exists already" >&2
        exit 1
    fi
    if [ "$(git branch --show-current)" != main ]; then
        echo "release from main" >&2
        exit 1
    fi
    if ! git diff --quiet HEAD; then
        echo "commit or stash your changes first" >&2
        exit 1
    fi
    git fetch -q origin main
    if [ "$(git rev-parse HEAD)" != "$(git rev-parse origin/main)" ]; then
        echo "main is not the same as origin/main: pull or push first" >&2
        exit 1
    fi

    echo "📦 Releasing v{{VERSION}} (CalVer)"

    # A stop before the commit puts the files back, so a rerun gets the same version.
    trap 'git checkout -q -- Cargo.toml Cargo.lock CHANGELOG.md; rm -f Cargo.toml.bak' EXIT

    # Update Cargo.toml version (only in [package] section), then Cargo.lock
    sed -i.bak '/^\[package\]/,/^\[/{s/^version = ".*"/version = "{{VERSION}}"/;}' Cargo.toml
    rm Cargo.toml.bak
    cargo check --quiet

    # Ensure it compiles and passes checks
    just check

    # Update CHANGELOG.md
    git-cliff --tag "v{{VERSION}}" -o CHANGELOG.md
    rumdl fmt CHANGELOG.md

    # A merge or the Changelog workflow can push to main while the checks run.
    git fetch -q origin main
    if [ "$(git rev-parse HEAD)" != "$(git rev-parse origin/main)" ]; then
        echo "origin/main moved while the checks ran: pull, then release again" >&2
        exit 1
    fi

    # Commit, tag, and push. cliff.toml leaves this subject out of the changelog.
    git add Cargo.toml Cargo.lock CHANGELOG.md
    git commit -m "chore(release): prepare for v{{VERSION}}"
    git tag "v{{VERSION}}"
    # One atomic push: the Changelog workflow must see the tag with the commit.
    git push --atomic origin main "v{{VERSION}}"

    echo "✅ Released v{{VERSION}}: the Release workflow builds it and publishes it to crates.io"

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
