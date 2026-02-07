# Data Model: cor — JSON Log Colorizer

**Feature**: `001-log-colorizer` | **Date**: 2026-02-07 (updated with clarifications)

---

## Entities

### LogLine

A single line read from stdin. Can be pure JSON, non-JSON text, or text with an embedded JSON object.

| Field | Type | Description |
|-------|------|-------------|
| `raw` | `String` | The original line text as read from stdin |
| `kind` | `LineKind` | Discriminant: `Json(LogRecord)`, `EmbeddedJson { prefix: String, record: LogRecord }`, or `Raw` |

**LineKind variants**:

| Variant | Description |
|---------|-------------|
| `Json(LogRecord)` | Entire line is valid JSON |
| `EmbeddedJson { prefix, record }` | Line starts with non-JSON text followed by a `{` that begins a valid JSON object. The `prefix` is the text before the opening brace. Example: `2026-02-06 00:15:13.449 {"level":"debug",...}` |
| `Raw` | Line contains no valid JSON — passed through unmodified |

**Embedded JSON detection** (FR-014):

1. If the line does not start with `{`, scan for the first `{` character.
2. If found, attempt to parse from that position to end-of-line as JSON.
3. If valid JSON object → `EmbeddedJson` with prefix = text before `{`.
4. If invalid → `Raw`.

**Validation**: None — all lines are valid (JSON or passthrough).

### LogRecord

A structured log entry extracted from a valid JSON line or embedded JSON object.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `timestamp` | `Option<Timestamp>` | No | Parsed timestamp value, if a known timestamp field was found |
| `level` | `Option<Level>` | No | Parsed log level, if a known level field was found |
| `message` | `Option<String>` | No | Extracted message text, if a known message field was found |
| `extra` | `BTreeMap<String, serde_json::Value>` | Yes | All remaining fields not consumed by timestamp/level/message extraction. Ordered alphabetically for deterministic output. Nested JSON objects are flattened 1 level using dot-notation (e.g., `{"http":{"method":"GET"}}` → key `http.method`). JSON arrays are NOT flattened — rendered as raw text. |

**Validation**:

- At least one of `timestamp`, `level`, or `message` SHOULD be present for meaningful output.
- If none are present, all fields are rendered as key=value pairs (treated as unstructured JSON).

### Level

Canonical log level enumeration.

| Variant | Numeric Value | Display | Color |
|---------|--------------|---------|-------|
| `Trace` | 10 | `TRACE` | Dim gray |
| `Debug` | 20 | `DEBUG` | Cyan |
| `Info` | 30 | `INFO` | Green |
| `Warn` | 40 | `WARN` | Yellow |
| `Error` | 50 | `ERROR` | Red |
| `Fatal` | 60 | `FATAL` | Bold red / red background |

**State transitions**: N/A — levels are immutable values.

**Validation**:

- String matching is case-insensitive.
- Unknown string values map to `None` (not an error). When displayed, `None` level renders as a blank badge (5 whitespace characters) to maintain column alignment.
- Numeric values outside the known range use nearest-match rounding (e.g., 25 → Debug, 35 → Info, 45 → Warn).

### FieldAliases

Static lookup table mapping canonical field concepts to known key names.

| Concept | Type | Known Aliases |
|---------|------|---------------|
| `timestamp` | `&[&str]` | `time`, `ts`, `timestamp`, `@timestamp`, `datetime`, `date`, `t`, `logged_at`, `created_at` |
| `level` | `&[&str]` | `level`, `severity`, `loglevel`, `log_level`, `lvl`, `priority`, `log.level` |
| `message` | `&[&str]` | `msg`, `message`, `text`, `log`, `body`, `event`, `short_message` |
| `logger` | `&[&str]` | `logger`, `name`, `logger_name`, `component`, `module` |
| `caller` | `&[&str]` | `caller`, `source`, `src`, `location`, `file`, `func`, `function` |
| `error` | `&[&str]` | `error`, `err`, `exception`, `exc_info`, `stack_trace`, `stacktrace`, `stack` |

**Lookup order**: First match wins. Aliases are ordered by frequency of use across frameworks.

### Timestamp

Parsed and normalized timestamp representation.

| Field | Type | Description |
|-------|------|-------------|
| `value` | `jiff::Timestamp` | Normalized timestamp value |
| `original` | `String` | Original string representation for fallback display |

**Supported input formats**:

- ISO 8601 / RFC 3339 strings
- Unix epoch seconds (integer or float)
- Unix epoch milliseconds (integer)
- Unix epoch nanoseconds (integer)
- `YYYY-MM-DD HH:MM:SS` format

**Heuristic for numeric timestamps**:

- Value < 1e12 → seconds
- Value < 1e15 → milliseconds
- Value ≥ 1e15 → nanoseconds

### Config

Application configuration merged from defaults, config file, and CLI arguments.

| Field | Type | Default | Source |
|-------|------|---------|--------|
| `color_mode` | `ColorMode` | `Auto` | `--color` flag |
| `min_level` | `Option<Level>` | `None` (show all) | `--level` flag |
| `message_key` | `Option<String>` | `None` (auto-detect) | `--message-key` flag / config |
| `level_key` | `Option<String>` | `None` (auto-detect) | `--level-key` flag / config |
| `timestamp_key` | `Option<String>` | `None` (auto-detect) | `--timestamp-key` flag / config |
| `include_fields` | `Option<Vec<String>>` | `None` (show all) | `--include-fields` flag |
| `exclude_fields` | `Option<Vec<String>>` | `None` (exclude none) | `--exclude-fields` flag |
| `json_output` | `bool` | `false` | `--json` flag |
| `max_field_length` | `Option<usize>` | `120` | `--max-field-length` flag / config |
| `timestamp_format` | `String` | `%H:%M:%S%.3f` | config file |
| `level_aliases` | `HashMap<String, Level>` | built-in mappings | config file |

**Merge precedence** (highest to lowest):

1. CLI flags
2. Config file (`$XDG_CONFIG_HOME/cor/config.toml`)
3. Built-in defaults

### ColorMode

Controls when ANSI colors are emitted.

| Variant | Behavior |
|---------|----------|
| `Auto` | Colors enabled if stdout is a TTY, `NO_COLOR` is not set, and `TERM` ≠ `dumb` |
| `Always` | Colors always enabled (overrides `NO_COLOR` and TTY detection) |
| `Never` | Colors never emitted |

---

## Relationships

```text
Config ──────────────────────┐
                             ▼
stdin ──▶ LogLine ──┬──▶ LogRecord ──▶ Formatter ──▶ stdout
                    │        │              ▲
                    │        ▼              │
                    │   FieldAliases       Level
                    │   (lookup)        (color map)
                    │        │
                    │        ▼
                    │   Timestamp
                    │   (normalize)
                    │
                    ├──▶ [EmbeddedJson: prefix + record]
                    │         └──▶ same LogRecord→Formatter pipeline
                    │              (prefix prepended to output)
                    │
                    └──▶ [Raw passthrough]
```

- `Config` is loaded once at startup and passed immutably to all processing.
- `LogLine` is created per stdin line and immediately consumed (no buffering).
- `FieldAliases` is a static, compile-time constant — no runtime allocation.
- `Level` ordering enables `>=` comparison for filtering.
- `EmbeddedJson` lines reuse the same `LogRecord` → `Formatter` pipeline, with the `prefix` prepended to the formatted output.
- Dot-notation flattening (`extra` fields): only 1 level deep — nested objects beyond the first level are rendered as raw JSON text.
- Field value truncation: values longer than `max_field_length` are truncated with `…` suffix.
