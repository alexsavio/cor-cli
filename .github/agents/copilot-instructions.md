# cor Development Guidelines

Auto-generated from all feature plans. Last updated: 2026-02-07

## Active Technologies
- Rust 1.75+ (2021 edition) + serde_json 1 (JSON parsing), clap 4 derive (CLI args), owo-colors 4 with supports-colors (terminal coloring, zero-alloc), jiff 0.1 (timestamp parsing), toml 0.8 (config file), thiserror 2 (error types). Optional: simd-json 0.17 behind feature flag. (001-log-colorizer)
- N/A (streaming stdin→stdout, optional XDG config file `~/.config/cor/config.toml`) (001-log-colorizer)

- Rust 1.75+ (2021 edition) + serde_json (JSON parsing), clap (CLI), owo-colors (terminal colors), jiff (timestamps), toml (config), thiserror (errors) (001-log-colorizer)

## Project Structure

```text
src/
tests/
```

## Commands

cargo test [ONLY COMMANDS FOR ACTIVE TECHNOLOGIES][ONLY COMMANDS FOR ACTIVE TECHNOLOGIES] cargo clippy

## Code Style

Rust 1.75+ (2021 edition): Follow standard conventions

## Recent Changes
- 001-log-colorizer: Added Rust 1.75+ (2021 edition) + serde_json 1 (JSON parsing), clap 4 derive (CLI args), owo-colors 4 with supports-colors (terminal coloring, zero-alloc), jiff 0.1 (timestamp parsing), toml 0.8 (config file), thiserror 2 (error types). Optional: simd-json 0.17 behind feature flag.

- 001-log-colorizer: Added Rust 1.75+ (2021 edition) + serde_json (JSON parsing), clap (CLI), owo-colors (terminal colors), jiff (timestamps), toml (config), thiserror (errors)

<!-- MANUAL ADDITIONS START -->
<!-- MANUAL ADDITIONS END -->
