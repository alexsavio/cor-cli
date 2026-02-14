# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

`cor` is a Rust CLI tool that colorizes JSON-structured log lines from stdin. It auto-detects timestamp/level/message fields across logging frameworks (logrus, zap, slog, pino, bunyan, structlog) and outputs colorized human-readable text. Non-JSON lines pass through unchanged.

- **Language:** Rust (edition 2024, MSRV 1.92)
- **Binary:** `cor`
- **Crate:** `cor` on crates.io

## Commands

```sh
# Build
cargo build
cargo build --release

# Run all tests
cargo test

# Run specific test
cargo test TEST_NAME -- --nocapture

# Lint
cargo clippy --all-targets --all-features -- -D warnings

# Format
cargo fmt

# Combined quality check (format + lint + test)
just check

# Run benchmarks
cargo bench

# Run demo
just demo
```

## Architecture

| File | Purpose |
|------|---------|
| `src/main.rs` | CLI entry point, stdin/stdout I/O loop, multi-line JSON reassembly |
| `src/cli.rs` | Clap argument definitions |
| `src/config.rs` | Configuration merging: defaults → TOML file (`~/.config/cor/config.toml`) → CLI flags |
| `src/parser.rs` | JSON log line parser with auto-detection and embedded JSON support |
| `src/formatter.rs` | Colorized output formatter |
| `src/level.rs` | Log level enum with parsing, display, colorization, and numeric level support |
| `src/timestamp.rs` | Timestamp parsing and formatting |
| `src/fields.rs` | Field alias tables for auto-detecting common log fields |
| `src/error.rs` | Error types using `thiserror` |

### Key Data Flow

1. **Input:** Lines read from stdin
2. **Parsing:** `parser::parse_line()` returns `LineKind` (JSON/EmbeddedJSON/Raw)
3. **Filtering:** Level filtering applied via `Config`
4. **Formatting:** `formatter::format_line_parsed()` produces colorized output
5. **Output:** Written to stdout with configurable line gap

### Multi-line JSON Handling

The main loop in `main.rs` handles JSON with embedded newlines by buffering up to 200 continuation lines and using `sanitize_json_newlines()` to reassemble them.

## Quality Requirements

After every change, **all must pass** before the work is done:

1. `cargo fmt -- --check`
2. `cargo clippy --all-targets --all-features -- -D warnings`
3. `cargo test`

Or simply run: `just check`

## Testing

- Integration tests in `tests/` use `assert_cmd` with fixtures in `tests/fixtures/`
- Unit tests are co-located within source modules
- Benchmarks in `benches/` use Criterion

## Release Process

Uses CalVer versioning (YYYY.MM.MICRO):

```sh
just release-next    # Auto-compute next version
just release 2026.2.6  # Explicit version
```
