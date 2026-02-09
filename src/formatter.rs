//! Colorized output formatter for structured log entries.
//!
//! Formats parsed [`LogRecord`] entries into human-readable output following
//! the fblog visual style:
//! - Bold timestamps
//! - Colored level badges with colon separator (`INFO:`)
//! - Plain message text
//! - Extra fields on separate lines with right-justified keys
//! - Field value truncation at configurable max length
//! - Include/exclude field filtering
//! - JSON passthrough mode

use std::fmt::Write;

use owo_colors::OwoColorize;

use crate::config::Config;
use crate::level::Level;
use crate::parser::{self, LineKind, LogRecord};

/// Format a single line for output.
///
/// If the line is JSON or embedded JSON, format it as colorized output.
/// If it's raw text, pass through unchanged.
/// If `--json` mode is active, output raw JSON (suppress non-JSON lines).
///
/// The result is written into `out`.
pub fn format_line(line: &str, config: &Config, use_color: bool, out: &mut String) {
    let parsed = parser::parse_line(line, config);
    format_line_parsed(parsed, line, config, use_color, out);
}

/// Format a pre-parsed [`LineKind`] for output.
///
/// Like [`format_line`], but accepts an already-parsed [`LineKind`] instead of
/// a raw line string. The `raw_line` parameter is used for `LineKind::Raw`
/// passthrough.
pub fn format_line_parsed(
    parsed: LineKind,
    raw_line: &str,
    config: &Config,
    use_color: bool,
    out: &mut String,
) {
    match parsed {
        LineKind::Json(record) => {
            if should_filter(&record, config) {
                // Line filtered out — signal empty output
                out.clear();
                return;
            }
            if config.json_output {
                out.push_str(&record.raw_json);
            } else {
                format_record(&record, None, config, use_color, out);
            }
        }
        LineKind::EmbeddedJson { prefix, record } => {
            if should_filter(&record, config) {
                out.clear();
                return;
            }
            if config.json_output {
                out.push_str(&record.raw_json);
            } else {
                format_record(&record, Some(&prefix), config, use_color, out);
            }
        }
        LineKind::Raw => {
            if config.json_output {
                // Non-JSON lines suppressed in --json mode
                out.clear();
                return;
            }
            // Pass through unchanged
            out.push_str(raw_line);
        }
    }
}

/// Check if a record should be filtered out by level.
fn should_filter(record: &LogRecord, config: &Config) -> bool {
    if let Some(ref min_level) = config.min_level {
        match &record.level {
            Some(level) => level < min_level,
            // No level field → show the line (can't evaluate)
            None => false,
        }
    } else {
        false
    }
}

/// Minimum width for extra field key alignment (right-justified).
const KEY_MIN_WIDTH: usize = 25;

/// Format a [`LogRecord`] into colorized human-readable output.
///
/// Output follows fblog style:
/// ```text
/// HH:MM:SS.mmm  INFO: message text
///                           key: value
///                     other_key: other_value
/// ```
fn format_record(
    record: &LogRecord,
    prefix: Option<&str>,
    config: &Config,
    use_color: bool,
    out: &mut String,
) {
    // Timestamp (bold when colored)
    if let Some(ref ts) = record.timestamp {
        let ts_str = ts.format_with(&config.timestamp_format);
        if use_color {
            let _ = write!(out, "{}  ", ts_str.bold());
        } else {
            out.push_str(&ts_str);
            out.push_str("  ");
        }
    }

    // Level badge + colon
    if let Some(ref level) = record.level {
        let badge = level.badge();
        if use_color {
            let style = level.style();
            let _ = write!(out, "{}:", badge.style(style));
        } else {
            out.push_str(badge);
            out.push(':');
        }
    } else {
        out.push_str(Level::blank_badge());
        out.push(':');
    }

    // Prefix (bold cyan when colored)
    if let Some(pfx) = prefix {
        if use_color {
            let _ = write!(out, " {}", pfx.bold().cyan());
        } else {
            out.push(' ');
            out.push_str(pfx);
        }
    }

    // Message (plain text, no bold)
    if let Some(ref msg) = record.message {
        out.push(' ');
        out.push_str(msg);
    }

    // Extra fields — each on a new line with right-justified key
    let max_len = config.max_field_length;

    for (key, value) in &record.extra {
        // Apply include/exclude filtering
        if let Some(ref include) = config.include_fields
            && !include.iter().any(|f| f == key)
        {
            continue;
        }
        if let Some(ref exclude) = config.exclude_fields
            && exclude.iter().any(|f| f == key)
        {
            continue;
        }

        let val_str = format_value(value);
        let val_display = truncate_value(&val_str, max_len);

        if use_color {
            let _ = write!(
                out,
                "\n{}: {}",
                format!("{key:>KEY_MIN_WIDTH$}")
                    .truecolor(150, 150, 150)
                    .bold(),
                val_display
            );
        } else {
            let _ = write!(out, "\n{key:>KEY_MIN_WIDTH$}: {val_display}");
        }
    }
}

/// Format a JSON value for display.
///
/// - Strings: unquoted
/// - Numbers/bools: as-is
/// - Arrays: compact JSON
/// - Objects: compact JSON (deeper nesting)
/// - Null: "null"
fn format_value(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Null => "null".to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Number(n) => n.to_string(),
        // Arrays and deep objects: compact JSON
        other => other.to_string(),
    }
}

/// Truncate a value string to `max_len` characters, appending `…` if truncated.
///
/// If `max_len` is `0`, no truncation is applied.
fn truncate_value(s: &str, max_len: usize) -> String {
    if max_len == 0 || s.chars().count() <= max_len {
        return s.to_string();
    }
    let truncated: String = s.chars().take(max_len).collect();
    format!("{truncated}…")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_value_no_truncation() {
        assert_eq!(truncate_value("hello", 120), "hello");
    }

    #[test]
    fn test_truncate_value_at_limit() {
        let s = "a".repeat(120);
        assert_eq!(truncate_value(&s, 120), s);
    }

    #[test]
    fn test_truncate_value_over_limit() {
        let s = "a".repeat(130);
        let result = truncate_value(&s, 120);
        assert_eq!(result.chars().count(), 121); // 120 + '…'
        assert!(result.ends_with('…'));
    }

    #[test]
    fn test_truncate_value_disabled() {
        let s = "a".repeat(1000);
        assert_eq!(truncate_value(&s, 0), s);
    }

    #[test]
    fn test_format_value_string() {
        let val = serde_json::json!("hello");
        assert_eq!(format_value(&val), "hello");
    }

    #[test]
    fn test_format_value_number() {
        let val = serde_json::json!(42);
        assert_eq!(format_value(&val), "42");
    }

    #[test]
    fn test_format_value_array() {
        let val = serde_json::json!([1, 2, 3]);
        assert_eq!(format_value(&val), "[1,2,3]");
    }

    #[test]
    fn test_format_value_null() {
        let val = serde_json::json!(null);
        assert_eq!(format_value(&val), "null");
    }

    #[test]
    fn test_format_line_raw_passthrough() {
        let config = Config::default();
        let mut out = String::new();
        format_line("plain text line", &config, false, &mut out);
        assert_eq!(out, "plain text line");
    }

    #[test]
    fn test_format_line_json_no_color() {
        let config = Config::default();
        let mut out = String::new();
        let line = r#"{"level":"info","msg":"hello","port":8080}"#;
        format_line(line, &config, false, &mut out);
        assert!(out.contains("INFO"));
        assert!(out.contains("hello"));
        assert!(out.contains("port: 8080"));
    }

    #[test]
    fn test_format_line_json_output_mode() {
        let config = Config {
            json_output: true,
            ..Config::default()
        };
        let mut out = String::new();
        let line = r#"{"level":"info","msg":"hello"}"#;
        format_line(line, &config, false, &mut out);
        assert_eq!(out, r#"{"level":"info","msg":"hello"}"#);
    }

    #[test]
    fn test_format_line_json_suppresses_raw() {
        let config = Config {
            json_output: true,
            ..Config::default()
        };
        let mut out = String::new();
        format_line("plain text", &config, false, &mut out);
        assert!(out.is_empty());
    }

    #[test]
    fn test_level_filtering() {
        let config = Config {
            min_level: Some(Level::Warn),
            ..Config::default()
        };

        // Info should be filtered
        let mut out = String::new();
        format_line(
            r#"{"level":"info","msg":"hello"}"#,
            &config,
            false,
            &mut out,
        );
        assert!(out.is_empty());

        // Warn should pass
        out.clear();
        format_line(
            r#"{"level":"warn","msg":"warning"}"#,
            &config,
            false,
            &mut out,
        );
        assert!(out.contains("warning"));

        // Raw always passes
        out.clear();
        format_line("plain text", &config, false, &mut out);
        assert_eq!(out, "plain text");
    }

    #[test]
    fn test_format_line_colorized_output() {
        let config = Config::default();
        let mut out = String::new();
        let line = r#"{"level":"info","msg":"hello"}"#;
        format_line(line, &config, true, &mut out);
        // Should contain ANSI escape sequences
        assert!(
            out.contains("\x1b["),
            "expected ANSI escapes in colorized output"
        );
        // Content should still be present
        assert!(out.contains("hello"));
    }

    #[test]
    fn test_exclude_fields() {
        let config = Config {
            exclude_fields: Some(vec!["port".to_string()]),
            ..Config::default()
        };
        let mut out = String::new();
        let line = r#"{"level":"info","msg":"hello","port":8080,"host":"localhost"}"#;
        format_line(line, &config, false, &mut out);
        assert!(
            !out.contains("port"),
            "excluded field 'port' should not appear"
        );
        assert!(
            out.contains("host"),
            "non-excluded field 'host' should appear"
        );
    }

    #[test]
    fn test_include_fields() {
        let config = Config {
            include_fields: Some(vec!["port".to_string()]),
            ..Config::default()
        };
        let mut out = String::new();
        let line = r#"{"level":"info","msg":"hello","port":8080,"host":"localhost"}"#;
        format_line(line, &config, false, &mut out);
        assert!(out.contains("port"), "included field 'port' should appear");
        assert!(
            !out.contains("host"),
            "non-included field 'host' should not appear"
        );
    }

    #[test]
    fn test_max_field_length_applied_in_format_line() {
        let config = Config {
            max_field_length: 10,
            ..Config::default()
        };
        let mut out = String::new();
        let long_value = "a".repeat(30);
        let line = format!(r#"{{"level":"info","msg":"hi","data":"{long_value}"}}"#);
        format_line(&line, &config, false, &mut out);
        // The truncated value should end with '…' and be shorter than the original
        assert!(out.contains('…'), "long field value should be truncated");
        assert!(!out.contains(&long_value), "full value should not appear");
    }

    #[test]
    fn test_timestamp_format_applied_in_format_line() {
        let config = Config {
            timestamp_format: "%H:%M:%S".to_string(),
            ..Config::default()
        };
        let mut out = String::new();
        let line = r#"{"level":"info","msg":"hi","time":"2026-01-15T10:30:00.123Z"}"#;
        format_line(line, &config, false, &mut out);
        assert!(
            out.contains("10:30:00"),
            "custom timestamp format should be applied"
        );
        // Should NOT contain a date since the format is time-only
        assert!(
            !out.contains("2026-01-15"),
            "date should not appear with time-only format"
        );
    }

    #[test]
    fn test_null_level_treated_as_absent() {
        let config = Config::default();
        let mut out = String::new();
        let line = r#"{"level":null,"msg":"hello"}"#;
        format_line(line, &config, false, &mut out);
        // Should use blank badge (5 spaces) since level is null
        assert!(
            out.contains("     :"),
            "null level should produce blank badge"
        );
        assert!(out.contains("hello"));
    }

    #[test]
    fn test_null_message_treated_as_absent() {
        let config = Config::default();
        let mut out = String::new();
        let line = r#"{"level":"info","msg":null,"port":8080}"#;
        format_line(line, &config, false, &mut out);
        assert!(out.contains("INFO"));
        assert!(out.contains("port"));
    }

    #[test]
    fn test_format_value_bool() {
        let val = serde_json::json!(true);
        assert_eq!(format_value(&val), "true");
        let val = serde_json::json!(false);
        assert_eq!(format_value(&val), "false");
    }

    #[test]
    fn test_format_value_object() {
        let val = serde_json::json!({"a": 1});
        assert_eq!(format_value(&val), r#"{"a":1}"#);
    }

    #[test]
    fn test_embedded_json_no_color() {
        let config = Config::default();
        let mut out = String::new();
        let line = r#"2026-02-06 prefix {"level":"debug","msg":"check"}"#;
        format_line(line, &config, false, &mut out);
        assert!(out.contains("DEBUG"));
        assert!(out.contains("check"));
        assert!(out.contains("2026-02-06 prefix"));
    }

    #[test]
    fn test_truncate_value_multibyte_characters() {
        // Emoji characters are multi-byte but count as 1 char each
        let s = "Hello \u{1F600}\u{1F600}\u{1F600} world";
        let result = truncate_value(s, 8);
        // Should truncate after 8 chars: "Hello 😀😀" + "…"
        assert!(result.ends_with('…'));
        assert_eq!(result.chars().count(), 9); // 8 + '…'
    }

    #[test]
    fn test_truncate_value_cjk_characters() {
        let s = "\u{4F60}\u{597D}\u{4E16}\u{754C}"; // 你好世界
        let result = truncate_value(s, 2);
        assert_eq!(result, "\u{4F60}\u{597D}\u{2026}"); // 你好…
    }

    #[test]
    fn test_format_line_no_timestamp_no_level_no_message() {
        let config = Config::default();
        let mut out = String::new();
        let line = r#"{"port":8080,"host":"localhost"}"#;
        format_line(line, &config, false, &mut out);
        // Should produce a blank badge and only extra fields
        assert!(out.contains("     :"), "should have blank badge");
        assert!(out.contains("port: 8080"));
        assert!(out.contains("host: localhost"));
    }

    #[test]
    fn test_level_filtering_embedded_json() {
        let config = Config {
            min_level: Some(Level::Error),
            ..Config::default()
        };

        // Info-level embedded JSON should be filtered
        let mut out = String::new();
        format_line(
            r#"prefix {"level":"info","msg":"hello"}"#,
            &config,
            false,
            &mut out,
        );
        assert!(out.is_empty(), "info should be filtered when min=error");

        // Error-level embedded JSON should pass
        out.clear();
        format_line(
            r#"prefix {"level":"error","msg":"fail"}"#,
            &config,
            false,
            &mut out,
        );
        assert!(out.contains("fail"), "error should pass when min=error");
    }

    #[test]
    fn test_format_line_json_mode_embedded() {
        // --json mode with embedded JSON should output only the JSON part
        let config = Config {
            json_output: true,
            ..Config::default()
        };
        let mut out = String::new();
        format_line(
            r#"prefix {"level":"info","msg":"hello"}"#,
            &config,
            false,
            &mut out,
        );
        // Should output the raw JSON, not the prefix
        assert!(out.starts_with('{'));
        assert!(out.contains("\"level\":\"info\""));
    }

    #[test]
    fn test_include_nonexistent_field() {
        // Including a field that doesn't exist should hide all extra fields
        let config = Config {
            include_fields: Some(vec!["nonexistent".to_string()]),
            ..Config::default()
        };
        let mut out = String::new();
        let line = r#"{"level":"info","msg":"hello","port":8080}"#;
        format_line(line, &config, false, &mut out);
        assert!(
            !out.contains("port"),
            "non-included fields should be hidden"
        );
    }
}
