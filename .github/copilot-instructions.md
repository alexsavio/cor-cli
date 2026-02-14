# Copilot Instructions for cor-cli

## Project overview

`cor` is a Rust CLI tool that colorizes JSON-structured log lines from stdin. It reads newline-delimited JSON log entries, auto-detects timestamp/level/message fields across major logging frameworks (logrus, zap, slog, pino, bunyan, structlog), and outputs colorized human-readable text to stdout. Non-JSON lines pass through unchanged. It also supports embedded JSON (lines with a non-JSON prefix before a JSON object).

- **Language:** Rust (edition 2024, MSRV 1.92)
- **Binary:** `cor`
- **Repository:** <https://github.com/alexsavio/cor-cli>

## Architecture

- `src/main.rs` — CLI entry point, stdin/stdout I/O loop, multi-line JSON reassembly
- `src/cli.rs` — Clap argument definitions
- `src/config.rs` — Configuration merging (defaults → TOML file → CLI flags)
- `src/parser.rs` — JSON log line parser with auto-detection and embedded JSON support
- `src/formatter.rs` — Colorized output formatter
- `src/level.rs` — Log level enum with parsing, display, and colorization
- `src/timestamp.rs` — Timestamp parsing and formatting
- `src/fields.rs` — Field alias tables for auto-detecting common log fields
- `src/error.rs` — Error types
- `tests/` — Integration tests using `assert_cmd` and fixtures in `tests/fixtures/`
- `specs/` — Design specifications and contracts

## Quality checks

After every task, **all of the following must pass** before considering the work done:

```sh
# Run all three, or use the combined check:
just check
```

Which runs:

1. **Format check:** `cargo fmt -- --check`
2. **Linter:** `cargo clippy --all-targets --all-features -- -D warnings`
3. **Tests:** `cargo test`

Do not leave code that fails any of these checks.
