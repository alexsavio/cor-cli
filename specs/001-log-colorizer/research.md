# Research: cor — JSON Log Colorizer CLI

**Date**: 2026-02-06 | **Spec**: `specs/001-log-colorizer`

---

## 1. Existing Tools in This Space

### 1.1 jq

| Attribute | Details |
|---|---|
| **Language** | C |
| **Stars** | 30k+ |
| **Key Features** | General-purpose JSON processor; powerful query/filter language; supports streaming with `--stream`; syntax highlighting of JSON output; can reshape/transform JSON arbitrarily |
| **Limitations** | Not log-specific — no awareness of log levels, timestamps, or message fields. No auto-detection of log structure. Requires users to write jq expressions for formatting. No level-based colorization. Steep learning curve for the filter language. |

### 1.2 humanlog

| Attribute | Details |
|---|---|
| **Language** | Go |
| **Stars** | ~914 |
| **Key Features** | Reads JSON and logfmt from stdin, pretty-prints to stdout. Auto-detects log structure. Supports Zap development format. Now also a localhost observability platform with local persistence, query engine (`humanlog query "spans | where duration > 100ms"`), and OpenTelemetry/OTLP trace ingestion. Color themes configurable via `--color` flag. |
| **Limitations** | Evolved beyond a simple formatter into a full observability platform — heavier than needed for pure colorization. Official releases include proprietary query engine code. Limited customization of output format. Go-based (no single static binary without CGO considerations). |

### 1.3 fblog

| Attribute | Details |
|---|---|
| **Language** | Rust |
| **Stars** | ~544 |
| **Key Features** | JSON log viewer written in Rust. Auto-detects message, severity, and timestamp fields. Lua-based filtering (`fblog -f 'level ~= "info"'`). Handlebars-based output format customization (`--main-line-format`). Supports field selection (`-a`), prefix parsing for kubectl (`-p`). Configuration via TOML file. Supports `NO_COLOR`. Custom level mappings (e.g., bunyan numeric levels). Placeholder substitution in messages. Shell completions. Nested field access. |
| **Limitations** | Lua dependency adds complexity. File-oriented (no native tailing — relies on `tail -f | fblog`). Limited streaming optimization for very high throughput. Handlebars formatting has a learning curve. |

### 1.4 jl (JSON Logs)

| Attribute | Details |
|---|---|
| **Language** | Go |
| **Stars** | ~246 |
| **Key Features** | Translates JSON structured logs into traditional log lines. Dynamically parses many well-known formats. `--color`/`--no-color` flags. Field control: `--skip-fields`, `--max-field-length`, `--include-fields`, `--exclude-fields`. Supports `--skip-prefix` / `--skip-suffix` for non-JSON text around JSON. Env var `JL_OPTS` for defaults. Handles nested message objects. |
| **Limitations** | Very lightweight — limited formatting customization. No filtering capability. No configuration file support. No custom level mappings. Small community (4 contributors, last release 2023). |

### 1.5 lnav (The Logfile Navigator)

| Attribute | Details |
|---|---|
| **Language** | C++ |
| **Stars** | 7k+ |
| **Key Features** | Full TUI log viewer. Merges multiple log files by timestamp into a single view. Auto-detects log formats (syslog, JSON-lines, logfmt, glog, W3C, etc.). SQL queries against log data via SQLite virtual tables. Timeline/histogram view. Pretty-print view for XML/JSON. Regex filters. Themes. Tab-completion. Session persistence. Custom keymaps. Headless mode for scripting. GZIP/BZIP2 decompression. |
| **Limitations** | Interactive TUI — not a simple stdin/stdout pipe tool. Heavy and complex. Overkill for simple "pipe and colorize" use case. Not designed for streaming from stdin as primary mode. |

### 1.6 bunyan CLI (Node.js)

| Attribute | Details |
|---|---|
| **Language** | JavaScript (Node.js) |
| **Stars** | ~7.2k (the bunyan package) |
| **Key Features** | Companion CLI for the Bunyan logging library. Renders bunyan JSON to colorized human-readable output. Level filtering (`bunyan -l warn`). Condition filtering (`bunyan -c 'this.lang == "fr"'`). Multiple output modes (short, long, json, etc.). Understands Bunyan's specific field schema (name, hostname, pid, level, msg, time, v). Renders `req`/`res`/`err` objects specially. |
| **Limitations** | Tightly coupled to Bunyan's specific log format. Requires Node.js runtime. Slow startup for a CLI tool. Not suitable for non-Bunyan JSON logs without adaptation. |

### 1.7 pino-pretty

| Attribute | Details |
|---|---|
| **Language** | JavaScript (Node.js) |
| **Stars** | ~1.6k |
| **Key Features** | Prettifier for Pino (ndjson) logs. Highly configurable: custom message/level/timestamp keys, translate time formats, ignore/include fields, custom colors, custom level names, message format templates with conditionals, custom prettifier functions per field, single-line mode, minimum level filtering. Config file support (`.pino-prettyrc`). TTY color detection. |
| **Limitations** | Tightly coupled to Pino's format. Requires Node.js. Slow startup. Not portable as a standalone binary. Shell limitations with stdout redirection (e.g., mingw64). |

### Summary Comparison

| Tool | Language | Stdin Pipe | Auto-detect | Filtering | Customizable Format | Config File | NO_COLOR |
|---|---|---|---|---|---|---|---|
| jq | C | Yes | No | Yes (jq lang) | Yes (jq lang) | No | No |
| humanlog | Go | Yes | Yes | No | Limited | No | Yes |
| fblog | Rust | Yes | Yes | Yes (Lua) | Yes (Handlebars) | Yes (TOML) | Yes |
| jl | Go | Yes | Yes | No | Limited | No | Yes |
| lnav | C++ | Limited | Yes | Yes (regex/SQL) | Yes | Yes | N/A |
| bunyan CLI | Node.js | Yes | Bunyan only | Yes (JS expr) | Modes only | No | No |
| pino-pretty | Node.js | Yes | Pino only | Level only | Yes (template) | Yes | Yes |

**Key gap for cor**: No Rust-based tool offers the combination of: fast single-binary, universal auto-detection of log formats, simple stdin→stdout pipeline, configurable output, and high performance for large log streams.

---

## 2. Structured Log Formats

### 2.1 Common Fields Across Frameworks

| Field Purpose | logrus (Go) | zap (Go) | slog (Go) | structlog (Python) | pino (Node.js) | bunyan (Node.js) |
|---|---|---|---|---|---|---|
| **Timestamp** | `time` (RFC3339) | `ts` (epoch float) | `time` (RFC3339) | `timestamp` (ISO8601) | `time` (epoch ms int) | `time` (ISO8601) |
| **Level** | `level` (string: `info`) | `level` (string: `info`) | `level` (string: `INFO`) | `level` (string: `info`) | `level` (int: `30`) | `level` (int: `30`) |
| **Message** | `msg` | `msg` | `msg` | `event` | `msg` | `msg` |
| **Logger name** | — | `logger` | — | `logger` | `name` | `name` |
| **Caller** | — (opt `func`) | `caller` (file:line) | `source` (obj) | — | — | `src` (obj) |
| **Stacktrace** | — | `stacktrace` | — | `stack_info` | — | `err.stack` |
| **Error** | `error` | `error` | — | `exc_info` | `err` | `err` |
| **PID** | — | — | — | — | `pid` | `pid` |
| **Hostname** | — | — | — | — | `hostname` | `hostname` |
| **Version** | — | — | — | — | `v` (1) | `v` (0) |

### 2.2 Canonical Field Name Aliases

A robust parser should recognize these common aliases:

| Concept | Known Field Names |
|---|---|
| **Timestamp** | `time`, `ts`, `timestamp`, `@timestamp`, `datetime`, `date`, `t`, `logged_at`, `created_at` |
| **Level/Severity** | `level`, `severity`, `loglevel`, `log_level`, `lvl`, `priority`, `log.level` |
| **Message** | `msg`, `message`, `text`, `log`, `body`, `event`, `short_message` (GELF) |
| **Logger** | `logger`, `name`, `logger_name`, `component`, `module` |
| **Caller/Source** | `caller`, `source`, `src`, `location`, `file`, `func`, `function` |
| **Error** | `error`, `err`, `exception`, `exc_info`, `stack_trace`, `stacktrace`, `stack` |

### 2.3 Level Value Mappings

| Level Name | bunyan/pino numeric | syslog numeric | Common string variants |
|---|---|---|---|
| TRACE | 10 | 7 (debug) | `trace`, `TRACE`, `trc` |
| DEBUG | 20 | 7 | `debug`, `DEBUG`, `dbg` |
| INFO | 30 | 6 | `info`, `INFO`, `inf`, `information` |
| WARN | 40 | 4 | `warn`, `WARN`, `warning`, `WARNING`, `wrn` |
| ERROR | 50 | 3 | `error`, `ERROR`, `err`, `ERR`, `fatal_error` |
| FATAL | 60 | 0-2 | `fatal`, `FATAL`, `critical`, `CRITICAL`, `crit`, `panic`, `PANIC`, `emerg` |

### 2.4 Timestamp Formats to Recognize

| Format | Example | Used By |
|---|---|---|
| ISO 8601 / RFC 3339 | `2024-01-15T10:30:00.123Z` | logrus, slog, bunyan, structlog |
| ISO 8601 with offset | `2024-01-15T10:30:00.123+02:00` | Various |
| Unix epoch seconds (float) | `1705312200.123` | zap |
| Unix epoch milliseconds (int) | `1705312200123` | pino |
| Unix epoch seconds (int) | `1705312200` | Various |
| Unix epoch nanoseconds (int) | `1705312200123456789` | Some Go loggers |
| RFC 2822 | `Mon, 15 Jan 2024 10:30:00 +0000` | Rare |
| Custom (`YYYY-MM-DD HH:MM:SS`) | `2024-01-15 10:30:00` | Python logging |

---

## 3. Terminal Colorization Best Practices

### 3.1 ANSI Escape Code Tiers

| Tier | Codes | Colors | Support |
|---|---|---|---|
| **Basic (3/4-bit)** | `\x1b[30m`–`\x1b[37m`, `\x1b[90m`–`\x1b[97m` | 16 colors (8 normal + 8 bright) | Universal — all terminals |
| **256-color (8-bit)** | `\x1b[38;5;{n}m` | 256 colors (216 color cube + 24 grays) | Very wide — xterm, iTerm2, most modern terminals |
| **Truecolor (24-bit)** | `\x1b[38;2;{r};{g};{b}m` | 16.7M colors | Modern terminals — iTerm2, kitty, WezTerm, Windows Terminal, GNOME Terminal 3.18+ |

### 3.2 Color Scheme Recommendations for Log Levels

| Level | Suggested Color | Rationale |
|---|---|---|
| TRACE | Dim/Gray | Lowest priority, should fade into background |
| DEBUG | Cyan/Blue | Informational but not primary |
| INFO | Green/Default | Normal operation, easy on the eyes |
| WARN | Yellow/Bold Yellow | Attention-getting but not alarming |
| ERROR | Red/Bold Red | Immediately signals problems |
| FATAL | Red background / Bold Magenta | Maximum contrast for critical failures |

### 3.3 NO_COLOR Standard (<https://no-color.org/>)

**Specification** (informal standard, proposed 2017):
> Command-line software which adds ANSI color to its output by default should check for a `NO_COLOR` environment variable that, when present and not an empty string (regardless of its value), prevents the addition of ANSI color.

**Implementation rules**:

1. Check `NO_COLOR` env var. If present and non-empty → disable all color output.
2. User-level config files and CLI flags (e.g., `--color=always`) should **override** `NO_COLOR`.
3. `NO_COLOR` only applies to ANSI color escape codes, not to other styling like bold/underline/italic.
4. Software that doesn't add color by default need not implement `NO_COLOR`.

**Recommended precedence** (highest to lowest):

1. `--color=always|never|auto` CLI flag
2. Config file setting
3. `NO_COLOR` / `FORCE_COLOR` env vars
4. TTY detection (is stdout a terminal?)
5. `TERM=dumb` check

### 3.4 Graceful Degradation When Piped

- **TTY detection**: Use `std::io::IsTerminal` (Rust 1.70+) to check if stdout is a terminal. If not, disable colors by default.
- **TERM=dumb**: If `TERM` is set to `dumb`, disable colors.
- **Piped output**: When piped to another program (not a TTY), emit plain text unless `--color=always` is specified.
- **Color capability detection**: Ideally detect 256-color/truecolor support via `COLORTERM=truecolor` or terminal-specific env vars, but basic ANSI is safest default.

### 3.5 Light/Dark Theme Considerations

- Avoid colors with poor contrast on either light or dark backgrounds (e.g., dark blue on black, yellow on white).
- Use **bright** variants of colors which generally work on both themes.
- Consider offering `--theme dark|light` or auto-detecting via `COLORFGBG` env var (limited).
- The safest approach: use dim/bright modifiers rather than specific colors that assume background.

---

## 4. Performance Considerations for Streaming CLI Tools

### 4.1 I/O Strategy

| Strategy | Throughput | Latency | Notes |
|---|---|---|---|
| **Line-by-line unbuffered** | Low | Low | Syscall per line; only for <100 lines/sec |
| **BufReader + BufWriter** | High | Moderate | Standard approach. 8KB default buffer. Flush on newline or when buffer full. |
| **Custom large buffer** | Highest | Higher | 64KB–256KB buffers for bulk processing. Higher latency for individual lines. |
| **Lock stdout once** | High | Low | `stdout().lock()` avoids per-write mutex acquisition |

### 4.2 Recommended Architecture

```text
stdin (BufReader 8KB)
  → read_line() into reusable String buffer
  → attempt JSON parse (serde_json::from_str or simd_json)
    → if valid JSON: extract known fields, format + colorize
    → if not JSON: pass through unchanged
  → write to stdout (BufWriter, locked)
```

### 4.3 Key Performance Techniques

1. **Reuse allocations**: Pre-allocate a `String` buffer for `read_line()` and `.clear()` between lines rather than allocating per line.
2. **Avoid unnecessary serialization**: Parse JSON, extract needed fields, format directly to output — don't serialize back to JSON.
3. **Lock stdout once**: Acquire `stdout().lock()` once at program start, not per line.
4. **BufWriter**: Wrap the locked stdout in `BufWriter` to batch writes. Flush on exit or every N lines for interactive responsiveness.
5. **Selective parsing**: For known field extraction, consider partial parsing (access only needed keys) rather than full DOM construction.
6. **Avoid allocations in hot path**: Use `write!()` macros directly to output rather than building intermediate `String`s.
7. **SIMD JSON**: For very high throughput (>100k lines/sec), `simd-json` can parse 2-4x faster than `serde_json`, but requires `&mut [u8]` input (in-place parsing).
8. **Memory**: Each line is independent — process and discard. No need to buffer multiple lines. Memory usage should be O(max_line_length), not O(total_input).

### 4.4 Benchmarking Targets

Based on the constitution's performance requirements:

- **Throughput target**: ≥100k lines/sec for typical JSON log lines (~200-500 bytes each)
- **Latency target**: <2ms per line visible on terminal for interactive use
- **Memory**: <50MB RSS regardless of input size (streaming, bounded)
- **Startup**: <100ms to first output

---

## 5. Recommended Rust Crates

### 5.1 JSON Parsing

| Crate | Recommendation | Justification |
|---|---|---|
| **`serde_json`** | **Primary** | Industry standard. Safe, well-tested, ubiquitous. `serde_json::Value` for dynamic JSON access. `from_str()` is fast enough for most log processing (~300-500MB/s). Zero-hassle API. |
| **`simd-json`** | **Optional / Future** | 2-4x faster than serde_json on supported architectures (x86 AVX2/SSE4.2, ARM NEON). Serde-compatible API. However: requires `&mut [u8]` input (in-place parsing), uses `unsafe` extensively, adds complexity. Best as opt-in feature flag for performance-critical deployments. |

**Recommendation**: Start with `serde_json`. Add `simd-json` behind a feature flag later if benchmarks show JSON parsing is the bottleneck.

### 5.2 Terminal Coloring

| Crate | Recommendation | Justification |
|---|---|---|
| **`owo-colors`** | **Primary** | Zero-allocation, `no_std` compatible, zero-cost. Supports all color tiers (ANSI 4-bit, Xterm 256, Truecolor RGB). Supports `NO_COLOR`/`FORCE_COLOR` via `supports-colors` feature. TTY detection. 91M+ downloads. Clean API with method chaining (`"text".red().bold()`). Compile-time and runtime color selection. |
| **`termcolor`** | Alternative | By BurntSushi (author of ripgrep). Excellent Windows console support. `BufferWriter` for thread-safe output. Supports `NO_COLOR` and `TERM=dumb`. 343M+ downloads. However: more verbose API (imperative `set_color` calls), no Truecolor support. |
| **`crossterm`** | Not recommended for this | Full terminal manipulation library (cursor, events, screen). Overkill for just colorized text output. Adds unnecessary weight. |
| **`colored`** | Not recommended | Allocating (creates new strings). Less actively maintained. No `no_std`. |
| **`anstream`** | Worth considering | Used by `clap`. Auto-detects color support. Works with any `Write` impl. Pairs well with `anstyle` for style definitions. |

**Recommendation**: `owo-colors` with the `supports-colors` feature for auto-detection. Its zero-allocation approach aligns with the performance goals.

### 5.3 CLI Argument Parsing

| Crate | Recommendation | Justification |
|---|---|---|
| **`clap`** (derive) | **Primary** | De facto standard for Rust CLIs. Derive macro for declarative argument definitions. Shell completion generation. `--help`/`--version` built-in. Subcommand support. Aligns with constitution's requirement for `--help`, `--version`, and structured output flags. |

### 5.4 Stdin Streaming / I/O

| Crate | Recommendation | Justification |
|---|---|---|
| **`std::io::BufRead`** | **Primary** | Standard library `BufReader<Stdin>` with `.read_line()` is sufficient and zero-dependency. Provides 8KB buffering by default. |
| **`std::io::BufWriter`** | **Primary** | Wrap `stdout().lock()` in `BufWriter` for batched output writes. |
| **`std::io::IsTerminal`** | **Primary** | Stable since Rust 1.70. Use for TTY detection to decide color mode. |

No external crate needed — the standard library covers streaming I/O well.

### 5.5 Other Recommended Crates

| Crate | Purpose | Justification |
|---|---|---|
| **`chrono`** or **`jiff`** | Timestamp parsing/formatting | Handle the variety of timestamp formats (ISO 8601, epoch seconds/ms/ns, etc.). `jiff` is newer and preferred for its correctness. |
| **`serde`** (derive) | Deserialization | Required by `serde_json`. Use for config file structs. |
| **`toml`** | Config file parsing | For `config.toml` configuration. Aligns with Rust ecosystem conventions. |
| **`thiserror`** | Error types | Derive `Error` for clean, actionable error messages per constitution. |

### 5.6 Crate Dependency Summary

```toml
[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
clap = { version = "4", features = ["derive"] }
owo-colors = { version = "4", features = ["supports-colors"] }
jiff = "0.1"
toml = "0.8"
thiserror = "2"

[features]
simd = ["simd-json"]

[dependencies.simd-json]
version = "0.17"
optional = true
```

---

## 6. Architecture Sketch

```text
┌──────────┐    ┌───────────┐    ┌──────────────┐    ┌───────────┐
│  stdin   │───▶│ BufReader │───▶│ Line Parser  │───▶│ Formatter │──▶ stdout
│ (pipe)   │    │ (8KB buf) │    │              │    │ (colorize)│   (BufWriter)
└──────────┘    └───────────┘    │ ┌──────────┐ │    └───────────┘
                                 │ │serde_json│ │
                                 │ │from_str()│ │
                                 │ └──────────┘ │
                                 │ ┌──────────┐ │
                                 │ │ Field    │ │
                                 │ │Extractor │ │
                                 │ └──────────┘ │
                                 └──────────────┘
```

**Flow**: Read line → Try parse as JSON → If JSON: extract timestamp, level, message, extra fields → Format with colors → Write. If not JSON: pass through unchanged.

---

## 7. Key Design Decisions for cor

| Decision | Recommendation | Rationale |
|---|---|---|
| **Output by default** | Colorized, human-readable | Primary use case |
| **Non-JSON lines** | Pass through unchanged | Don't break mixed-format log streams |
| **Color control** | `--color=auto\|always\|never` | Standard convention, overrides `NO_COLOR` |
| **Field auto-detection** | Alias tables (§2.2) | Support many frameworks without configuration |
| **Custom field mapping** | Config file + CLI flags | `--message-key`, `--level-key`, `--timestamp-key` |
| **Level filtering** | `--level warn` (show warn+) | Common need, simple implementation |
| **Extra fields** | Appended as `key=value` pairs | Like `jl` and humanlog |
| **Streaming** | Line-by-line, no buffering of multiple lines | Minimal latency for interactive use |
| **Config file** | `config.toml` in XDG config | Persistent preferences (colors, field mappings, level maps) |
