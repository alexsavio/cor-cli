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
use owo_colors::Stream::Stdout;

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
pub fn format_line(line: &str, config: &Config, out: &mut String) {
    let parsed = parser::parse_line(line, config);
    format_line_parsed(parsed, line, config, out);
}

/// Format a pre-parsed [`LineKind`] for output.
///
/// Like [`format_line`], but accepts an already-parsed [`LineKind`] instead of
/// a raw line string. The `raw_line` parameter is used for `LineKind::Raw`
/// passthrough.
pub fn format_line_parsed(parsed: LineKind, raw_line: &str, config: &Config, out: &mut String) {
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
                format_record(&record, None, config, out);
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
                format_record(&record, Some(&prefix), config, out);
            }
        }
        LineKind::Raw(parse_error) => {
            if config.json_output {
                // Non-JSON lines suppressed in --json mode
                out.clear();
                return;
            }
            // Pass through unchanged
            out.push_str(raw_line);

            // In verbose mode, show parse error if present
            if config.verbose
                && let Some(err) = parse_error
            {
                let _ = write!(
                    out,
                    "\n  {} [{}:{}] {}",
                    "parse error:".if_supports_color(Stdout, |t| t.red().bold().to_string()),
                    err.line,
                    err.column,
                    err.message
                        .if_supports_color(Stdout, |t| t.dimmed().to_string()),
                );
            }
        }
    }
}

/// Check if a record should be filtered out by level.
#[inline]
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

/// Format a [`LogRecord`] into colorized human-readable output.
///
/// Output follows fblog style:
/// ```text
/// HH:MM:SS.mmm  INFO: message text
///                           key: value
///                     other_key: other_value
/// ```
fn format_record(record: &LogRecord, prefix: Option<&str>, config: &Config, out: &mut String) {
    // Timestamp (bold when colored)
    if let Some(ref ts) = record.timestamp {
        let ts_str = ts.format_with(&config.timestamp_format);
        let _ = write!(
            out,
            "{}  ",
            ts_str.if_supports_color(Stdout, |t| t.bold().to_string())
        );
    }

    // Level badge + colon
    if let Some(ref level) = record.level {
        let badge = level.badge();
        let custom_color = config
            .level_colors
            .as_ref()
            .and_then(|colors| colors.get(level))
            .map(String::as_str);
        let style = level.style_with_color(custom_color);
        let _ = write!(
            out,
            "{}:",
            badge.if_supports_color(Stdout, |t| t.style(style).to_string())
        );
    } else {
        out.push_str(Level::blank_badge());
        out.push(':');
    }

    // Logger name (dimmed, after level badge)
    if let Some(ref logger) = record.logger {
        let _ = write!(
            out,
            " {}",
            logger.if_supports_color(Stdout, |t| t.dimmed().to_string())
        );
    }

    // Prefix (bold cyan when colored)
    if let Some(pfx) = prefix {
        let _ = write!(
            out,
            " {}",
            pfx.if_supports_color(Stdout, |t| t.bold().cyan().to_string())
        );
    }

    // Message (plain text, no bold)
    if let Some(ref msg) = record.message {
        out.push(' ');
        out.push_str(msg);
    }

    // Caller (dimmed, in parentheses after message)
    if let Some(ref caller) = record.caller {
        let _ = write!(
            out,
            " ({})",
            caller.if_supports_color(Stdout, |t| t.dimmed().to_string())
        );
    }

    // Extra fields — each on a new line with right-justified key
    let max_len = config.max_field_length;
    let key_width = config.key_min_width;

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

        let _ = write!(
            out,
            "\n{}: {}",
            format!("{key:>key_width$}")
                .if_supports_color(Stdout, |t| t.truecolor(150, 150, 150).bold().to_string()),
            val_display
        );
    }

    // Error field (red, with multiline support for stacktraces)
    if let Some(ref error) = record.error {
        format_error_field(error, key_width, out);
    }
}

/// Format the error field with red styling and multiline stacktrace support.
fn format_error_field(error: &str, key_width: usize, out: &mut String) {
    let label = format!("{:>key_width$}", "error");
    let styled_label = label.if_supports_color(Stdout, |t| t.red().bold().to_string());

    if error.contains('\n') {
        // Multiline error: indent continuation lines to align with value column
        let indent = " ".repeat(key_width + 2); // key_width + ": "
        let mut lines = error.lines();
        if let Some(first) = lines.next() {
            let _ = write!(
                out,
                "\n{}: {}",
                styled_label,
                first.if_supports_color(Stdout, |t| t.red().to_string())
            );
            for line in lines {
                let _ = write!(
                    out,
                    "\n{}{}",
                    indent,
                    line.if_supports_color(Stdout, |t| t.red().to_string())
                );
            }
        }
    } else {
        let _ = write!(
            out,
            "\n{}: {}",
            styled_label,
            error.if_supports_color(Stdout, |t| t.red().to_string())
        );
    }
}

/// Format a JSON value for display.
///
/// - Strings: unquoted
/// - Numbers/bools: as-is
/// - Arrays: compact JSON
/// - Objects: compact JSON (deeper nesting)
/// - Null: "null"
#[inline]
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
#[inline]
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

    fn disable_color() {
        owo_colors::set_override(false);
    }

    #[test]
    fn test_format_line_raw_passthrough() {
        disable_color();
        let config = Config::default();
        let mut out = String::new();
        format_line("plain text line", &config, &mut out);
        assert_eq!(out, "plain text line");
    }

    #[test]
    fn test_format_line_json_no_color() {
        disable_color();
        let config = Config::default();
        let mut out = String::new();
        let line = r#"{"level":"info","msg":"hello","port":8080}"#;
        format_line(line, &config, &mut out);
        assert!(out.contains("INFO"));
        assert!(out.contains("hello"));
        assert!(out.contains("port: 8080"));
    }

    #[test]
    fn test_format_line_json_output_mode() {
        disable_color();
        let config = Config {
            json_output: true,
            ..Config::default()
        };
        let mut out = String::new();
        let line = r#"{"level":"info","msg":"hello"}"#;
        format_line(line, &config, &mut out);
        assert_eq!(out, r#"{"level":"info","msg":"hello"}"#);
    }

    #[test]
    fn test_format_line_json_suppresses_raw() {
        disable_color();
        let config = Config {
            json_output: true,
            ..Config::default()
        };
        let mut out = String::new();
        format_line("plain text", &config, &mut out);
        assert!(out.is_empty());
    }

    #[test]
    fn test_level_filtering() {
        disable_color();
        let config = Config {
            min_level: Some(Level::Warn),
            ..Config::default()
        };

        // Info should be filtered
        let mut out = String::new();
        format_line(r#"{"level":"info","msg":"hello"}"#, &config, &mut out);
        assert!(out.is_empty());

        // Warn should pass
        out.clear();
        format_line(r#"{"level":"warn","msg":"warning"}"#, &config, &mut out);
        assert!(out.contains("warning"));

        // Raw always passes
        out.clear();
        format_line("plain text", &config, &mut out);
        assert_eq!(out, "plain text");
    }

    // Colorized output is tested via integration tests (color_always_enables_ansi,
    // embedded_json_colorized) which control the --color flag end-to-end.

    #[test]
    fn test_exclude_fields() {
        disable_color();
        let config = Config {
            exclude_fields: Some(vec!["port".to_string()]),
            ..Config::default()
        };
        let mut out = String::new();
        let line = r#"{"level":"info","msg":"hello","port":8080,"host":"localhost"}"#;
        format_line(line, &config, &mut out);
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
        disable_color();
        let config = Config {
            include_fields: Some(vec!["port".to_string()]),
            ..Config::default()
        };
        let mut out = String::new();
        let line = r#"{"level":"info","msg":"hello","port":8080,"host":"localhost"}"#;
        format_line(line, &config, &mut out);
        assert!(out.contains("port"), "included field 'port' should appear");
        assert!(
            !out.contains("host"),
            "non-included field 'host' should not appear"
        );
    }

    #[test]
    fn test_max_field_length_applied_in_format_line() {
        disable_color();
        let config = Config {
            max_field_length: 10,
            ..Config::default()
        };
        let mut out = String::new();
        let long_value = "a".repeat(30);
        let line = format!(r#"{{"level":"info","msg":"hi","data":"{long_value}"}}"#);
        format_line(&line, &config, &mut out);
        // The truncated value should end with '…' and be shorter than the original
        assert!(out.contains('…'), "long field value should be truncated");
        assert!(!out.contains(&long_value), "full value should not appear");
    }

    #[test]
    fn test_timestamp_format_applied_in_format_line() {
        disable_color();
        let config = Config {
            timestamp_format: "%H:%M:%S".to_string(),
            ..Config::default()
        };
        let mut out = String::new();
        let line = r#"{"level":"info","msg":"hi","time":"2026-01-15T10:30:00.123Z"}"#;
        format_line(line, &config, &mut out);
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
        disable_color();
        let config = Config::default();
        let mut out = String::new();
        let line = r#"{"level":null,"msg":"hello"}"#;
        format_line(line, &config, &mut out);
        // Should use blank badge (5 spaces) since level is null
        assert!(
            out.contains("     :"),
            "null level should produce blank badge"
        );
        assert!(out.contains("hello"));
    }

    #[test]
    fn test_null_message_treated_as_absent() {
        disable_color();
        let config = Config::default();
        let mut out = String::new();
        let line = r#"{"level":"info","msg":null,"port":8080}"#;
        format_line(line, &config, &mut out);
        assert!(out.contains("INFO"));
        assert!(out.contains("port"));
    }

    #[test]
    fn test_embedded_json_no_color() {
        disable_color();
        let config = Config::default();
        let mut out = String::new();
        let line = r#"2026-02-06 prefix {"level":"debug","msg":"check"}"#;
        format_line(line, &config, &mut out);
        assert!(out.contains("DEBUG"));
        assert!(out.contains("check"));
        assert!(out.contains("2026-02-06 prefix"));
    }

    #[test]
    fn test_format_line_no_timestamp_no_level_no_message() {
        disable_color();
        let config = Config::default();
        let mut out = String::new();
        let line = r#"{"port":8080,"host":"localhost"}"#;
        format_line(line, &config, &mut out);
        // Should produce a blank badge and only extra fields
        assert!(out.contains("     :"), "should have blank badge");
        assert!(out.contains("port: 8080"));
        assert!(out.contains("host: localhost"));
    }

    #[test]
    fn test_level_filtering_embedded_json() {
        disable_color();
        let config = Config {
            min_level: Some(Level::Error),
            ..Config::default()
        };

        // Info-level embedded JSON should be filtered
        let mut out = String::new();
        format_line(
            r#"prefix {"level":"info","msg":"hello"}"#,
            &config,
            &mut out,
        );
        assert!(out.is_empty(), "info should be filtered when min=error");

        // Error-level embedded JSON should pass
        out.clear();
        format_line(
            r#"prefix {"level":"error","msg":"fail"}"#,
            &config,
            &mut out,
        );
        assert!(out.contains("fail"), "error should pass when min=error");
    }

    #[test]
    fn test_format_line_json_mode_embedded() {
        disable_color();
        // --json mode with embedded JSON should output only the JSON part
        let config = Config {
            json_output: true,
            ..Config::default()
        };
        let mut out = String::new();
        format_line(
            r#"prefix {"level":"info","msg":"hello"}"#,
            &config,
            &mut out,
        );
        // Should output the raw JSON, not the prefix
        assert!(out.starts_with('{'));
        assert!(out.contains("\"level\":\"info\""));
    }

    #[test]
    fn test_include_nonexistent_field() {
        disable_color();
        // Including a field that doesn't exist should hide all extra fields
        let config = Config {
            include_fields: Some(vec!["nonexistent".to_string()]),
            ..Config::default()
        };
        let mut out = String::new();
        let line = r#"{"level":"info","msg":"hello","port":8080}"#;
        format_line(line, &config, &mut out);
        assert!(
            !out.contains("port"),
            "non-included fields should be hidden"
        );
    }

    #[test]
    fn test_verbose_shows_parse_error() {
        disable_color();
        let config = Config {
            verbose: true,
            ..Config::default()
        };
        let mut out = String::new();
        let line = r#"{"level":"info", "msg":}"#; // Invalid JSON
        format_line(line, &config, &mut out);
        assert!(
            out.contains("parse error:"),
            "verbose mode should show parse error"
        );
        // Error message varies between serde_json and simd-json parsers
        assert!(
            out.len() > line.len(),
            "output should include error details: {out}"
        );
    }

    #[test]
    fn test_verbose_disabled_hides_parse_error() {
        disable_color();
        let config = Config {
            verbose: false,
            ..Config::default()
        };
        let mut out = String::new();
        let line = r#"{"level":"info", "msg":}"#; // Invalid JSON
        format_line(line, &config, &mut out);
        assert!(
            !out.contains("parse error"),
            "verbose off should not show parse error"
        );
        // Line should still pass through
        assert_eq!(out, line);
    }

    #[test]
    fn test_verbose_no_error_for_plain_text() {
        disable_color();
        let config = Config {
            verbose: true,
            ..Config::default()
        };
        let mut out = String::new();
        format_line("plain text line", &config, &mut out);
        assert!(
            !out.contains("parse error"),
            "plain text should not show parse error"
        );
        assert_eq!(out, "plain text line");
    }

    #[test]
    fn test_verbose_shows_error_for_embedded_malformed_json() {
        disable_color();
        let config = Config {
            verbose: true,
            ..Config::default()
        };
        let mut out = String::new();
        let line = r#"prefix {"broken":}"#;
        format_line(line, &config, &mut out);
        assert!(
            out.contains("parse error:"),
            "verbose should show error for malformed embedded JSON"
        );
    }

    #[test]
    fn test_level_filtering_no_level_passes_through() {
        disable_color();
        // JSON records with no recognized level field should pass through
        // even when min_level filtering is active.
        let config = Config {
            min_level: Some(Level::Error),
            ..Config::default()
        };
        let mut out = String::new();
        let line = r#"{"msg":"no level field","port":8080}"#;
        format_line(line, &config, &mut out);
        assert!(
            out.contains("no level field"),
            "JSON record without level should pass through when filtering is active"
        );
    }

    // ── format_error_field tests ────────────────────────────────────

    #[test]
    fn test_format_error_field_single_line() {
        disable_color();
        let mut out = String::new();
        format_error_field("connection timeout", 25, &mut out);
        assert!(
            out.contains("error"),
            "error label should appear.\nGot: {out}"
        );
        assert!(
            out.contains("connection timeout"),
            "error value should appear.\nGot: {out}"
        );
        // format_error_field prepends a newline for alignment under extra fields
        assert!(
            out.starts_with('\n'),
            "error field should start with newline.\nGot: {out}"
        );
        // Should contain exactly one newline (the leading one) — no continuation lines
        assert_eq!(
            out.matches('\n').count(),
            1,
            "single-line error should have only the leading newline.\nGot: {out}"
        );
    }

    #[test]
    fn test_format_error_field_multiline() {
        disable_color();
        let error = "Traceback:\n  File \"app.py\", line 72\n    raise Error";
        let mut out = String::new();
        format_error_field(error, 25, &mut out);
        assert!(
            out.contains("error"),
            "error label should appear.\nGot: {out}"
        );
        assert!(
            out.contains("Traceback:"),
            "first line of error should appear.\nGot: {out}"
        );
        assert!(
            out.contains("File \"app.py\""),
            "continuation lines should appear.\nGot: {out}"
        );
        assert!(
            out.contains("raise Error"),
            "last line of error should appear.\nGot: {out}"
        );
        // Continuation lines should be indented (key_width + ": " = 27 spaces)
        let indent = " ".repeat(27);
        assert!(
            out.contains(&format!("{indent}File")),
            "continuation lines should be indented.\nGot: {out}"
        );
    }

    // ── Logger and caller rendering ─────────────────────────────────

    #[test]
    fn test_format_record_with_logger() {
        disable_color();
        let config = Config::default();
        let mut out = String::new();
        let line = r#"{"level":"info","msg":"hello","logger":"payments.processor"}"#;
        format_line(line, &config, &mut out);
        assert!(
            out.contains("payments.processor"),
            "logger should appear in output.\nGot: {out}"
        );
        // Logger should NOT appear as a key-value extra field
        assert!(
            !out.contains("logger:"),
            "logger should not appear as extra field.\nGot: {out}"
        );
    }

    #[test]
    fn test_format_record_with_caller() {
        disable_color();
        let config = Config::default();
        let mut out = String::new();
        let line = r#"{"level":"info","msg":"hello","caller":"server/handler.go:42"}"#;
        format_line(line, &config, &mut out);
        assert!(
            out.contains("(server/handler.go:42)"),
            "caller should appear in parentheses.\nGot: {out}"
        );
        assert!(
            !out.contains("caller:"),
            "caller should not appear as extra field.\nGot: {out}"
        );
    }

    #[test]
    fn test_format_record_with_error() {
        disable_color();
        let config = Config::default();
        let mut out = String::new();
        let line = r#"{"level":"error","msg":"fail","error":"connection refused"}"#;
        format_line(line, &config, &mut out);
        assert!(
            out.contains("connection refused"),
            "error value should appear.\nGot: {out}"
        );
        // Error field appears after extra fields with "error:" label
        assert!(
            out.contains("error:"),
            "error label should appear.\nGot: {out}"
        );
    }
}
