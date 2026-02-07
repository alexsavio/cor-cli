# CLI Contract: cor

**Feature**: `001-log-colorizer` | **Date**: 2026-02-07 (updated with clarifications)

---

## Binary

**Name**: `cor`
**Description**: Colorize JSON-structured log lines from stdin
**Version**: `0.1.0`

---

## Usage

```text
cor [OPTIONS]
```

Reads JSON log lines from stdin, outputs colorized human-readable text to stdout. Non-JSON lines are passed through unchanged.

---

## Options

| Flag | Short | Type | Default | Description |
|------|-------|------|---------|-------------|
| `--color <MODE>` | `-c` | `auto\|always\|never` | `auto` | Control color output. `auto` enables colors only when stdout is a TTY and `NO_COLOR` is unset. |
| `--level <LEVEL>` | `-l` | `trace\|debug\|info\|warn\|error\|fatal` | (none — show all) | Minimum severity level to display. Lines below this level are suppressed. |
| `--message-key <KEY>` | `-m` | `string` | (auto-detect) | Override the JSON key used for the log message field. |
| `--level-key <KEY>` | | `string` | (auto-detect) | Override the JSON key used for the log level field. |
| `--timestamp-key <KEY>` | `-t` | `string` | (auto-detect) | Override the JSON key used for the timestamp field. |
| `--include-fields <FIELDS>` | `-i` | `comma-separated` | (all) | Only show these extra fields. Cannot be used with `--exclude-fields`. |
| `--exclude-fields <FIELDS>` | `-e` | `comma-separated` | (none) | Hide these extra fields. Cannot be used with `--include-fields`. |
| `--json` | `-j` | `flag` | `false` | Output filtered lines as JSON instead of colorized text. Useful for chaining with other tools. |
| `--max-field-length <N>` | `-M` | `integer` | `120` | Maximum character length for extra field values. Values exceeding this length are truncated with a trailing `…`. Set to `0` to disable truncation. |
| `--config <PATH>` | | `path` | `$XDG_CONFIG_HOME/cor/config.toml` | Path to configuration file. |
| `--help` | `-h` | `flag` | | Print help information. |
| `--version` | `-V` | `flag` | | Print version information. |

---

## Environment Variables

| Variable | Effect |
|----------|--------|
| `NO_COLOR` | When present and non-empty, disables all color output (overridden by `--color=always`). |
| `FORCE_COLOR` | When present and non-empty, forces color output even when not a TTY (overridden by `--color=never`). |
| `XDG_CONFIG_HOME` | Base directory for config file lookup. Defaults to `~/.config`. |
| `TERM` | If set to `dumb`, colors are disabled in `auto` mode. |

**Precedence** (highest to lowest):

1. `--color` CLI flag
2. Config file `color` setting
3. `NO_COLOR` / `FORCE_COLOR` env vars
4. TTY detection (`std::io::IsTerminal`)
5. `TERM=dumb` check

---

## Output Format

### Colorized (default)

```text
HH:MM:SS.mmm LEVEL message key1=value1 key2=value2
```

**Components**:

- **Timestamp**: Formatted as `HH:MM:SS.mmm` in dim text. Omitted if no timestamp field found.
- **Level**: 5-character padded badge (e.g., `INFO`, `WARN`, `ERROR`), colored per level. When no level field is found or the value is unrecognized, displayed as 5 whitespace characters to maintain column alignment.
- **Message**: The log message in default/bold text.
- **Extra fields**: Remaining JSON fields as `key=value` pairs in dim text, sorted alphabetically. String values unquoted. Nested objects are flattened 1 level using dot-notation (e.g., `http.method=GET`). Nested objects beyond 1 level and JSON arrays are rendered as compact raw JSON. Values exceeding `max_field_length` characters are truncated with `…`.
- **Prefix** (embedded JSON lines only): When a line contains non-JSON text before a JSON object, the prefix text is displayed before the formatted log record in dim text.

**Example input**:

```json
{"ts":"2026-01-15T10:30:00.123Z","level":"info","msg":"server started","port":8080,"host":"0.0.0.0"}
```

**Example output** (colors represented in text):

```text
10:30:00.123 INFO  server started host=0.0.0.0 port=8080
```

### JSON output (`--json`)

When `--json` is specified, output is the original JSON line unchanged (with level filtering applied if `--level` is also set). One JSON object per line. Non-JSON lines are **suppressed** in `--json` mode — only valid JSON objects are emitted, since the purpose is to produce machine-parseable output for downstream tooling.

### Non-JSON passthrough

Lines that fail JSON parsing (including embedded JSON detection) are written to stdout unchanged, uncolored.

**Behavior with `--level` filter** (FR-015): Non-JSON lines are **always displayed** regardless of the active level filter. Only JSON log records with a recognized level below the threshold are suppressed.

---

## Exit Codes

| Code | Meaning |
|------|---------|
| `0` | Success — stdin fully consumed |
| `1` | Configuration error (invalid flag combination, unreadable config file) |
| `2` | I/O error other than broken pipe (e.g., write failure to stdout due to disk full or permission denied) |

**Broken pipe handling**: When downstream consumer closes the pipe (e.g., `cor | head -5`), cor exits silently with code 0 (suppresses `BrokenPipe` error). Broken pipe is NOT an exit-code-2 scenario.

---

## Config File Format

**Path**: `$XDG_CONFIG_HOME/cor/config.toml` (default: `~/.config/cor/config.toml`)

```toml
# Default color mode: "auto", "always", or "never"
color = "auto"

# Default minimum level (optional)
# level = "info"

# Timestamp display format (strftime-compatible)
timestamp_format = "%H:%M:%S%.3f"

# Custom field key overrides
[keys]
message = "msg"        # Override message field key
level = "level"        # Override level field key
timestamp = "time"     # Override timestamp field key

# Custom level name mappings (extend built-in aliases)
[levels]
"information" = "info"
"verbose" = "debug"
"critical" = "fatal"

# Custom colors per level (ANSI color names)
[colors]
trace = "bright black"
debug = "cyan"
info = "green"
warn = "yellow"
error = "red"
fatal = "bright red"

# Maximum field value length (0 = unlimited)
max_field_length = 120
```

---

## Conflict Rules

- `--include-fields` and `--exclude-fields` are mutually exclusive → exit code 1 with error message.
- `--color=always` overrides `NO_COLOR`.
- `--color=never` overrides `FORCE_COLOR`.
- CLI flags override config file values.
- Unknown CLI flags → error with suggestion (clap default behavior).
