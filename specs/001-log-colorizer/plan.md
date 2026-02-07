# Implementation Plan: JSON Log Colorizer CLI

**Branch**: `001-log-colorizer` | **Date**: 2026-02-07 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/001-log-colorizer/spec.md`

## Summary

Build `cor`, a Rust CLI tool that reads JSON-structured log lines from stdin and outputs colorized, human-readable text to stdout. The tool auto-detects common log fields (timestamp, level, message) across major logging frameworks (logrus, zap, slog, pino, bunyan, structlog) using alias tables. It supports level-based filtering, embedded JSON detection in prefixed lines, dot-notation flattening of nested objects (1 level), field value truncation (`--max-field-length`, default 120), `NO_COLOR`/TTY auto-detection, and processes ≥100k lines/sec with bounded memory.

## Technical Context

**Language/Version**: Rust 1.75+ (2021 edition)
**Primary Dependencies**: serde 1 (derive), serde_json 1 (JSON parsing), clap 4 derive (CLI args), owo-colors 4 with supports-colors (terminal coloring, zero-alloc), jiff 0.1 (timestamp parsing), toml 0.8 (config file), thiserror 2 (error types). Optional: simd-json 0.17 behind feature flag.
**Storage**: N/A (streaming stdin→stdout, optional XDG config file `~/.config/cor/config.toml`)
**Testing**: cargo test (unit + integration), criterion (benchmarks)
**Target Platform**: Linux, macOS, Windows (cross-platform terminal support via owo-colors)
**Project Type**: single
**Performance Goals**: ≥100k lines/sec throughput, <100ms startup to first output
**Constraints**: <50MB RSS regardless of input size, <2ms per-line latency for interactive use
**Scale/Scope**: Single binary CLI tool, ~2k-5k LOC estimated

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

### Pre-design gate (2026-02-06)

| Principle | Status | Evidence |
|-----------|--------|----------|
| **I. Code Quality First** | ✅ PASS | Single-responsibility modules planned (parser, formatter, config, cli, level, fields, timestamp, error). Clippy + rustfmt enforced. All public APIs documented with rustdoc. Functions kept under 40 lines. |
| **II. Testing Standards** | ✅ PASS | TDD workflow. Unit tests per module. Integration tests for end-to-end piping, level filtering, color control, mixed input, embedded JSON. Benchmark suite via criterion. >80% coverage target. |
| **III. UX Consistency** | ✅ PASS | `--help`, `--version`, `--color`, `--json` flags. Actionable error messages via thiserror. Consistent CLI argument pattern. `NO_COLOR` standard. Exit codes 0/1/2 defined. Broken pipe handled gracefully. |
| **IV. Performance Requirements** | ✅ PASS | Performance budgets: 100k lines/sec, <50MB RSS, <100ms startup. Benchmarks in CI via criterion. Streaming architecture prevents unbounded memory. Field value truncation at 120 chars default. |

### Post-design gate (2026-02-07)

| Principle | Status | Evidence |
|-----------|--------|----------|
| **I. Code Quality First** | ✅ PASS | 9 source modules, each with single responsibility. `parser.rs` handles both pure-JSON and embedded-JSON lines. `formatter.rs` handles dot-notation flattening and truncation. All entities documented in data-model.md. |
| **II. Testing Standards** | ✅ PASS | 5 integration test files (added `embedded_json.rs`). 6 framework fixture files + embedded.jsonl. Edge cases from clarification session all have concrete expected behavior suitable for test assertions. |
| **III. UX Consistency** | ✅ PASS | `--max-field-length` added to CLI contract. Unknown level → blank badge (5-char space). Non-JSON passthrough during filtering. Embedded prefix preserved in output. All behaviors documented in contracts/cli.md. |
| **IV. Performance Requirements** | ✅ PASS | Embedded JSON detection adds a constant-time `memchr('{')` scan — no performance regression. Dot-notation flattening is O(fields) per line — bounded. Truncation reduces output volume. |

## Project Structure

### Documentation (this feature)

```text
specs/001-log-colorizer/
├── plan.md              # This file
├── spec.md              # Feature specification (with clarifications)
├── research.md          # Phase 0 research output
├── data-model.md        # Phase 1 data model
├── quickstart.md        # Phase 1 quickstart guide
├── contracts/           # Phase 1 CLI interface contracts
│   └── cli.md           # CLI argument and output contracts
└── tasks.md             # Phase 2 output (created by /speckit.tasks)
```

### Source Code (repository root)

```text
src/
├── main.rs              # Entry point, CLI setup, I/O loop
├── cli.rs               # Clap argument definitions
├── config.rs            # TOML config loading, XDG paths
├── parser.rs            # JSON line parsing, field extraction, embedded JSON detection
├── formatter.rs         # Colorized output formatting, dot-notation flattening, truncation
├── level.rs             # Level enum, aliases, numeric mapping, colors
├── fields.rs            # Field alias tables, canonical field detection
├── timestamp.rs         # Timestamp format detection and normalization
└── error.rs             # Error types (thiserror)

tests/
├── integration/
│   ├── basic_pipe.rs    # End-to-end stdin→stdout tests
│   ├── level_filter.rs  # Level filtering tests (incl. non-JSON passthrough)
│   ├── color_control.rs # NO_COLOR, --color flag, TTY detection tests
│   ├── mixed_input.rs   # JSON + non-JSON mixed input tests
│   └── embedded_json.rs # Lines with non-JSON prefix + embedded JSON object
└── fixtures/
    ├── logrus.jsonl      # Sample logrus output
    ├── zap.jsonl         # Sample zap output
    ├── slog.jsonl        # Sample Go slog output
    ├── pino.jsonl        # Sample pino output
    ├── bunyan.jsonl      # Sample bunyan output
    ├── structlog.jsonl   # Sample structlog output
    └── embedded.jsonl    # Lines with prefix text + embedded JSON

benches/
└── throughput.rs         # Criterion benchmark for lines/sec
```

**Structure Decision**: Single project structure. This is a standalone CLI tool with no web frontend, API server, or mobile component. All source code lives under `src/`, tests under `tests/`, benchmarks under `benches/`.

## Complexity Tracking

No constitution violations — no entries required.
