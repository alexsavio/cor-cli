# Feature Specification: JSON Log Colorizer CLI

**Feature Branch**: `001-log-colorizer`
**Created**: 2026-02-06
**Status**: Draft
**Input**: User description: "Build a CLI application that colorizes the json lines from structured logs that are redirected to it."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Pipe JSON Logs and See Colorized Output (Priority: P1)

As a developer, I want to pipe JSON-structured log output from my application into `cor` and immediately see a human-readable, colorized version in my terminal, so I can quickly scan logs during development.

**Why this priority**: This is the core value proposition — without this, the tool has no purpose.

**Independent Test**: Run `echo '{"ts":"2026-01-15T10:30:00Z","level":"info","msg":"server started","port":8080}' | cor` and verify colorized output appears with timestamp, level badge, message, and extra fields.

**Acceptance Scenarios**:

1. **Given** a JSON log line on stdin, **When** `cor` processes it, **Then** it outputs a colorized, human-readable line with timestamp, level, message, and remaining fields.
2. **Given** a non-JSON line on stdin, **When** `cor` processes it, **Then** it passes the line through unchanged (no crash, no error).
3. **Given** JSON logs from different frameworks (zap, logrus, slog, pino, bunyan, structlog), **When** piped to `cor`, **Then** it auto-detects timestamp/level/message fields using known aliases.
4. **Given** `NO_COLOR` env var is set, **When** `cor` processes logs, **Then** output is plain text without ANSI escape codes.
5. **Given** stdout is piped to another program, **When** `cor` processes logs, **Then** colors are disabled by default.

---

### User Story 2 - Filter Logs by Level (Priority: P2)

As a developer, I want to filter log output by minimum severity level, so I can focus on warnings and errors during debugging.

**Why this priority**: Level filtering is the most common second action after viewing logs — reduces noise significantly.

**Independent Test**: Run `cat mixed-logs.jsonl | cor --level warn` and verify only WARN, ERROR, and FATAL lines appear.

**Acceptance Scenarios**:

1. **Given** a stream with mixed log levels, **When** `--level warn` is specified, **Then** only lines with severity >= WARN are shown.
2. **Given** numeric level values (bunyan/pino format), **When** filtering by level name, **Then** numeric values are correctly mapped and filtered.
3. **Given** `--level` is not specified, **When** `cor` processes logs, **Then** all levels are shown (no filtering).

---

### User Story 3 - Customize Field Display (Priority: P3)

As a developer, I want to control which fields are shown and override field key mappings, so I can tailor the output to my application's specific log format.

**Why this priority**: Customization is needed for non-standard log formats but not essential for initial usability.

**Independent Test**: Run `echo '{"custom_msg":"hello","sev":"INFO"}' | cor --message-key custom_msg --level-key sev` and verify correct parsing.

**Acceptance Scenarios**:

1. **Given** `--message-key`, `--level-key`, or `--timestamp-key` flags, **When** processing logs, **Then** the specified keys are used instead of auto-detection.
2. **Given** `--include-fields` or `--exclude-fields`, **When** processing logs, **Then** only the specified extra fields are shown or hidden.
3. **Given** a `config.toml` config file in `$XDG_CONFIG_HOME/cor/`, **When** `cor` starts, **Then** it uses the config for defaults (overridden by CLI flags).

---

### Edge Cases

- What happens when a JSON line has no recognized level field? → Leave the level badge position as blank whitespace (5 chars) for alignment; do not show a placeholder.
- What happens when a JSON line has no message field? → Display all fields as key=value pairs.
- What happens with extremely long lines (>1MB)? → Process them without unbounded memory growth; extra field values are truncated at 120 characters by default (configurable via `--max-field-length`; use `0` for no truncation).
- What happens with malformed JSON (partial, trailing comma)? → Pass through unchanged as a non-JSON line.
- What happens with nested JSON objects as field values? → Flatten one level using dot-notation (e.g., `headers.content-type=application/json`). Deeper nesting rendered as compact inline JSON.
- What happens with empty stdin? → Exit cleanly with code 0.
- What happens when stdin is a terminal (not a pipe)? → Read interactively, process each entered line.
- What happens with non-JSON lines when `--level` filter is active? → Non-JSON lines are always shown (passed through); they cannot be evaluated against a level filter.
- What happens with lines that have a non-JSON prefix followed by embedded JSON (e.g., `2026-02-06 00:15:13.449 {"level":"debug","msg":"..."}`)? → The parser MUST detect the embedded JSON object, extract and colorize it, and preserve the prefix text as-is.
- What happens with valid JSON arrays on stdin (e.g., `[1, 2, 3]`)? → Pass through as raw text. Only JSON objects (`{...}`) are parsed as log entries.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST read JSON lines from stdin and output colorized, human-readable text to stdout.
- **FR-002**: System MUST auto-detect common log fields (timestamp, level, message) using a built-in alias table covering logrus, zap, slog, pino, bunyan, and structlog formats.
- **FR-003**: System MUST pass through non-JSON lines unchanged without error. When `--level` filtering is active, non-JSON lines MUST still be displayed (non-JSON lines cannot be evaluated against a level filter). When `--json` mode is active, non-JSON lines MUST be suppressed.
- **FR-004**: System MUST support `--color=auto|always|never` flag with auto as default.
- **FR-005**: System MUST respect `NO_COLOR` env var per the no-color.org standard.
- **FR-006**: System MUST disable colors when stdout is not a TTY (unless `--color=always`).
- **FR-007**: System MUST support `--level <LEVEL>` to filter by minimum severity.
- **FR-008**: System MUST support `--help` and `--version` flags.
- **FR-009**: System MUST support custom field key overrides via `--message-key`, `--level-key`, `--timestamp-key`.
- **FR-010**: System MUST support `--include-fields` and `--exclude-fields` for controlling extra field display.
- **FR-011**: System MUST support a TOML configuration file for persistent preferences.
- **FR-012**: System MUST handle numeric log levels (bunyan/pino) by mapping to named levels.
- **FR-013**: System MUST produce structured JSON output when `--json` flag is provided (passthrough with optional filtering). Non-JSON lines are suppressed in `--json` mode.
- **FR-014**: System MUST detect lines with a non-JSON prefix followed by an embedded JSON object, extract and colorize the JSON portion, and preserve the prefix text as-is in the output.
- **FR-015**: *(Merged into FR-003)* — See FR-003 for non-JSON passthrough behavior under `--level` filtering.
- **FR-016**: System MUST truncate extra field values at 120 characters by default (with `…` ellipsis), configurable via `--max-field-length`. A value of `0` disables truncation.

### Key Entities

- **LogLine**: A single line from stdin — either a valid JSON object or raw text.
- **LogRecord**: Parsed representation of a JSON log line with extracted timestamp, level, message, and extra fields.
- **FieldAlias**: Mapping from a canonical field concept (timestamp, level, message) to known key names.
- **Level**: Canonical log level enumeration with bidirectional mapping between level names, numeric values, and display colors.
- **Config**: User configuration loaded from TOML file and CLI arguments.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Tool processes ≥100,000 typical JSON log lines per second on a modern machine (streaming, no backpressure).
- **SC-002**: Memory usage remains <50MB regardless of input size (streaming, bounded).
- **SC-003**: Startup to first output in <100ms.
- **SC-004**: Correctly auto-detects and formats logs from at least 6 major logging frameworks without configuration.
- **SC-005**: `cor --help` provides clear, complete usage information.
- **SC-006**: Non-JSON lines pass through without modification or error in 100% of cases.

## Clarifications

### Session 2026-02-07

- Q: When `--level` filter is active and a non-JSON line arrives, should it be shown or suppressed? → A: Always show non-JSON lines (pass through regardless of level filter). Non-JSON lines lack the context to filter, and suppressing them could drop important output like stack traces or startup banners.
- Q: Should lines with a non-JSON prefix followed by embedded JSON (e.g., `2026-02-06 00:15:13.449 {"level":"debug",...}`) be handled? → A: Yes. The parser must detect the embedded JSON object, extract and colorize it, and preserve the prefix text as-is.
- Q: How should nested JSON objects/arrays in extra fields be displayed? → A: Dot-notation flattening (1 level deep). E.g., `headers.content-type=application/json`. Deeper nesting rendered as compact inline JSON.
- Q: How should valid JSON arrays on stdin be handled? → A: Pass through as raw text. Only JSON objects (`{...}`) are parsed as log entries.
- Q: Should extra field values be truncated for display? → A: Yes, truncate at 120 characters by default with `…` ellipsis. Configurable via `--max-field-length` flag (0 = no truncation).
- Q: What should be displayed when no recognized level field is found in a JSON log line? → A: Leave the level badge position as blank whitespace (5 chars for alignment). No placeholder character.
- Q: How should non-JSON lines be handled in `--json` output mode? → A: Suppress them. `--json` mode emits only valid JSON objects for machine-parseable downstream consumption. Non-JSON lines are silently dropped in this mode.
