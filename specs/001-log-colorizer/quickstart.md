# Quickstart: cor — JSON Log Colorizer

## Prerequisites

- **Rust** 1.75+ with `cargo` ([install](https://rustup.rs/))

## Build & Install

```bash
# Clone and build
git clone <repo-url> && cd cor
cargo build --release

# Install to ~/.cargo/bin/
cargo install --path .
```

## Verify Installation

```bash
cor --version
# cor 0.1.0

cor --help
```

## Basic Usage

### Pipe structured logs

```bash
# From a running application
my-app 2>&1 | cor

# From a log file
cat app.log | cor

# Tail a log file
tail -f /var/log/app.jsonl | cor
```

### Sample input/output

```bash
echo '{"ts":"2026-01-15T10:30:00.123Z","level":"info","msg":"server started","port":8080}' | cor
# Output: 10:30:00.123 INFO  server started port=8080
```

### Filter by level

```bash
cat app.log | cor --level warn
# Shows only WARN, ERROR, and FATAL lines
```

### Disable colors (for piping)

```bash
cat app.log | cor --color never > clean.log
```

### Custom field keys

```bash
cat app.log | cor --message-key event --level-key severity --timestamp-key datetime
```

### Select specific fields

```bash
cat app.log | cor --include-fields user_id,request_id
# Or exclude noisy fields
cat app.log | cor --exclude-fields pid,hostname
```

### JSON passthrough with filtering

```bash
cat app.log | cor --json --level error > errors-only.jsonl
```

### Embedded JSON in prefixed lines

```bash
# Lines like: "2026-02-06 00:15:13.449 {"level":"debug","msg":"health check",...}"
# cor detects the JSON portion and colorizes it, preserving the prefix
my-app 2>&1 | cor
# Output: 2026-02-06 00:15:13.449 00:15:13.449 DEBUG health check ...
```

### Truncate long field values

```bash
# Default: truncate values longer than 120 characters with …
cat app.log | cor

# Custom length
cat app.log | cor --max-field-length 80

# Disable truncation
cat app.log | cor --max-field-length 0
```

## Run Tests

```bash
cargo test
```

## Run Benchmarks

```bash
cargo bench
```

## Configuration

Create `~/.config/cor/config.toml`:

```toml
timestamp_format = "%H:%M:%S%.3f"

[keys]
message = "msg"
level = "level"
timestamp = "time"
```

See [contracts/cli.md](contracts/cli.md) for full config file reference.
