//! Command-line argument definitions for `cor`.
//!
//! Uses [`clap`] derive macros for argument parsing. All flags are documented
//! in the contract specification at `specs/001-log-colorizer/contracts/cli.md`.

use clap::{Parser, ValueEnum};

/// Colorize JSON-structured log lines from stdin.
///
/// Reads JSON log lines from stdin, outputs colorized human-readable text
/// to stdout. Non-JSON lines are passed through unchanged.
#[derive(Debug, Parser)]
#[command(name = "cor", version, about, long_about = None)]
pub struct Cli {
    /// Control color output.
    ///
    /// `auto` enables colors only when stdout is a TTY and `NO_COLOR` is unset.
    #[arg(short = 'c', long, value_enum, default_value_t = ColorMode::Auto)]
    pub color: ColorMode,

    /// Minimum severity level to display.
    ///
    /// Lines below this level are suppressed. Non-JSON lines always pass through.
    #[arg(short = 'l', long, value_parser = parse_level_arg)]
    pub level: Option<String>,

    /// Override the JSON key used for the log message field.
    #[arg(short = 'm', long)]
    pub message_key: Option<String>,

    /// Override the JSON key used for the log level field.
    #[arg(long)]
    pub level_key: Option<String>,

    /// Override the JSON key used for the timestamp field.
    #[arg(short = 't', long)]
    pub timestamp_key: Option<String>,

    /// Only show these extra fields (comma-separated).
    ///
    /// Cannot be used with `--exclude-fields`.
    #[arg(
        short = 'i',
        long,
        value_delimiter = ',',
        conflicts_with = "exclude_fields"
    )]
    pub include_fields: Option<Vec<String>>,

    /// Hide these extra fields (comma-separated).
    ///
    /// Cannot be used with `--include-fields`.
    #[arg(
        short = 'e',
        long,
        value_delimiter = ',',
        conflicts_with = "include_fields"
    )]
    pub exclude_fields: Option<Vec<String>>,

    /// Output filtered lines as JSON instead of colorized text.
    ///
    /// Non-JSON lines are suppressed in this mode.
    #[arg(short = 'j', long)]
    pub json: bool,

    /// Maximum character length for extra field values.
    ///
    /// Values exceeding this length are truncated with `…`.
    /// Set to `0` to disable truncation.
    #[arg(short = 'M', long)]
    pub max_field_length: Option<usize>,

    /// Number of blank lines between each log entry.
    ///
    /// Set to `0` for compact output with no gaps.
    #[arg(short = 'g', long)]
    pub line_gap: Option<usize>,

    /// Path to configuration file.
    #[arg(long)]
    pub config: Option<std::path::PathBuf>,

    /// Show parse errors for lines that look like JSON but fail to parse.
    ///
    /// When enabled, lines starting with `{` that fail JSON parsing will
    /// display the `serde_json` error message after the raw line.
    #[arg(short = 'v', long)]
    pub verbose: bool,
}

/// Color output mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ColorMode {
    /// Enable colors only when stdout is a TTY.
    Auto,
    /// Always enable colors.
    Always,
    /// Never enable colors.
    Never,
}

/// Parse level argument as case-insensitive string.
fn parse_level_arg(s: &str) -> Result<String, String> {
    let lower = s.to_lowercase();
    match lower.as_str() {
        "trace" | "debug" | "info" | "warn" | "error" | "fatal" => Ok(lower),
        _ => Err(format!(
            "invalid level '{s}': expected one of trace, debug, info, warn, error, fatal"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_level_arg_valid() {
        assert_eq!(parse_level_arg("info").unwrap(), "info");
        assert_eq!(parse_level_arg("INFO").unwrap(), "info");
        assert_eq!(parse_level_arg("Warn").unwrap(), "warn");
        assert_eq!(parse_level_arg("TRACE").unwrap(), "trace");
        assert_eq!(parse_level_arg("debug").unwrap(), "debug");
        assert_eq!(parse_level_arg("error").unwrap(), "error");
        assert_eq!(parse_level_arg("fatal").unwrap(), "fatal");
    }

    #[test]
    fn test_parse_level_arg_invalid() {
        let err = parse_level_arg("verbose").unwrap_err();
        assert!(err.contains("invalid level"));
        let err = parse_level_arg("").unwrap_err();
        assert!(err.contains("invalid level"));
        let err = parse_level_arg("critical").unwrap_err();
        assert!(err.contains("invalid level"));
    }
}
