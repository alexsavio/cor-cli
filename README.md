# cor

Colorize JSON-structured log lines from stdin.

`cor` reads newline-delimited JSON log entries from stdin and prints colorized,
human-readable output to stdout. Non-JSON lines pass through unchanged.

```text
$ echo '{"level":"info","ts":"2026-01-15T10:30:01.456Z","msg":"server started","host":"0.0.0.0","port":8080}' | cor

2026-01-15T10:30:01.456   INFO: server started
                                host: 0.0.0.0
                                port: 8080
```

## Demo

### Default output

`cat logs.jsonl | cor`

![Default colorized output](https://raw.githubusercontent.com/alexsavio/cor-cli/main/assets/demo/01-default.png)

### Level filtering

`cat logs.jsonl | cor --level warn`

![Level filtering](https://raw.githubusercontent.com/alexsavio/cor-cli/main/assets/demo/02-level-filter.png)

### Include specific fields

`cat logs.jsonl | cor -i method,path,status`

![Include fields](https://raw.githubusercontent.com/alexsavio/cor-cli/main/assets/demo/03-include-fields.png)

### Exclude fields

`cat logs.jsonl | cor -e func,query`

![Exclude fields](https://raw.githubusercontent.com/alexsavio/cor-cli/main/assets/demo/04-exclude-fields.png)

### JSON passthrough

`cat logs.jsonl | cor --json --level error`

![JSON output](https://raw.githubusercontent.com/alexsavio/cor-cli/main/assets/demo/05-json-output.png)

### Field truncation

`cat logs.jsonl | cor --max-field-length 20`

![Truncate fields](https://raw.githubusercontent.com/alexsavio/cor-cli/main/assets/demo/06-truncate-fields.png)

## Features

- **Auto-detects fields** from logrus, zap, slog, pino, bunyan, structlog, and more
- **Embedded JSON** — handles lines like `2026-01-15 10:30:00 {"level":"info",...}`
- **Level filtering** — `--level warn` suppresses debug and info
- **Numeric levels** — bunyan/pino `30`→info, `40`→warn, etc.
- **Custom keys** — `--message-key`, `--level-key`, `--timestamp-key`
- **Field filtering** — `--include-fields` or `--exclude-fields`
- **JSON passthrough** — `--json` outputs filtered JSON for piping
- **Truncation** — long values truncated at 120 chars (configurable)
- **Config file** — `~/.config/cor/config.toml` for persistent settings
- **NO_COLOR** — respects [no-color.org](https://no-color.org) convention
- **Fast** — ~400K lines/sec, O(line-length) memory, streaming I/O

## Install

### From crates.io

```sh
cargo install cor
```

### From source

```sh
git clone https://github.com/alexsavio/cor-cli.git
cd cor-cli
cargo install --path .
```

## Usage

```sh
# Pipe any JSON log stream
my-app | cor

# Filter by level
kubectl logs my-pod | cor --level warn

# Custom keys
my-app | cor --message-key event --level-key severity

# Only show specific fields
my-app | cor --include-fields=host,port,status

# Hide noisy fields
my-app | cor --exclude-fields=pid,hostname

# Output filtered JSON (for piping)
my-app | cor --level error --json | jq .

# Disable truncation
my-app | cor --max-field-length=0

# Force colors in pipes
my-app | cor --color=always | less -R
```

## Output format

```text
YYYY-MM-DDTHH:MM:SS.mmm  LEVEL: message
                                      key: value
                                other_key: other_value
```

- **Timestamp** — bold `YYYY-MM-DDTHH:MM:SS.mmm` in UTC
- **Level** — colored and bold, right-justified in a 5-char field
  - <span style="color:cyan">TRACE</span> · <span style="color:blue">DEBUG</span> · <span style="color:green"> INFO</span> · <span style="color:yellow"> WARN</span> · <span style="color:red">ERROR</span> · <span style="color:magenta">FATAL</span>
- **Message** — plain text
- **Extra fields** — one per line, key right-justified to 25 chars, bold gray

## Log levels

These levels are recognized (case-insensitive, with aliases):

| Level | Aliases                        | Numeric (bunyan/pino) |
|-------|--------------------------------|-----------------------|
| TRACE | `trace`, `trc`                 | 10                    |
| DEBUG | `debug`, `dbg`                 | 20                    |
| INFO  | `info`, `inf`, `information`   | 30                    |
| WARN  | `warn`, `warning`, `wrn`       | 40                    |
| ERROR | `error`, `err`, `fatal_error`  | 50                    |
| FATAL | `fatal`, `critical`, `crit`, `panic`, `emerg` | 60 |

Custom level aliases can be defined in the config file.

## Auto-detected fields

`cor` scans for well-known field names used by popular logging frameworks:

| Field     | Aliases                                                            |
|-----------|--------------------------------------------------------------------|
| Timestamp | `time`, `ts`, `timestamp`, `@timestamp`, `datetime`, `date`, `t`  |
| Level     | `level`, `severity`, `loglevel`, `log_level`, `lvl`, `priority`   |
| Message   | `msg`, `message`, `text`, `log`, `body`, `event`, `short_message` |

CLI flags (`--message-key`, `--level-key`, `--timestamp-key`) override auto-detection.

## Embedded JSON

Lines with a text prefix before JSON are detected automatically:

```text
2026-01-15 10:30:00.123 {"level":"info","msg":"server started","port":8080}
[INFO] 2026-01-15T10:30:01Z {"level":"debug","msg":"config loaded"}
myapp | {"level":"warn","msg":"disk space low"}
```

The prefix is preserved in the output after the level badge.

## Nested objects

Nested JSON objects are flattened using dot notation:

```json
{"level":"info","msg":"req","http":{"method":"GET","status":200}}
```

```text
INFO: req
              http.method: GET
              http.status: 200
```

## Config file

`cor` loads `~/.config/cor/config.toml` (or `$XDG_CONFIG_HOME/cor/config.toml`) if present. CLI flags always take precedence.

```toml
# Default minimum level
level = "info"

# Color mode: auto, always, never
color = "auto"

# Timestamp display format (strftime)
timestamp_format = "%Y-%m-%dT%H:%M:%S%.3f"

# Max field value length (0 = unlimited)
max_field_length = 120

# Examples of custom timestamp formats:
# timestamp_format = "%H:%M:%S%.3f"    # time only with milliseconds
# timestamp_format = "%H:%M:%S"        # time only, no milliseconds

# Override field key names
[keys]
message = "msg"
level = "level"
timestamp = "ts"

# Map custom level names → standard levels
[levels]
"verbose" = "debug"
"critical" = "fatal"
"success" = "info"
```

## Environment variables

| Variable      | Effect                                         |
|---------------|-------------------------------------------------|
| `NO_COLOR`    | Disables colors when set (any non-empty value)  |
| `FORCE_COLOR` | Enables colors even when not a TTY              |
| `TERM=dumb`   | Disables colors in `auto` mode                  |

`--color=always` and `--color=never` override all environment variables.

## CLI reference

```text
cor [OPTIONS]

Options:
  -c, --color <COLOR>              Color mode [default: auto] [values: auto, always, never]
  -l, --level <LEVEL>              Minimum severity level [values: trace, debug, info, warn, error, fatal]
  -m, --message-key <KEY>          Override message field key
      --level-key <KEY>            Override level field key
  -t, --timestamp-key <KEY>        Override timestamp field key
  -i, --include-fields <FIELDS>    Only show these fields (comma-separated)
  -e, --exclude-fields <FIELDS>    Hide these fields (comma-separated)
  -j, --json                       Output raw JSON instead of colorized text
  -M, --max-field-length <N>       Max field value length [default: 120]
      --config <PATH>              Path to config file
  -h, --help                       Print help
  -V, --version                    Print version
```

## License

[MIT](LICENSE) — Alexandre Savio
