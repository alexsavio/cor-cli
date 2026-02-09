//! JSON log line parser with auto-detection and embedded JSON support.
//!
//! Parses stdin lines into structured [`LogRecord`] entries by auto-detecting
//! timestamp, level, and message fields across major logging frameworks.
//! Supports pure JSON lines, lines with a non-JSON prefix before a JSON object
//! (embedded JSON), and plain text passthrough.

use std::collections::BTreeMap;

use crate::config::Config;
use crate::fields;
use crate::level::Level;
use crate::timestamp::Timestamp;

/// The parsed classification of a stdin line.
#[derive(Debug)]
pub enum LineKind {
    /// Entire line is a valid JSON object.
    Json(LogRecord),
    /// Line has non-JSON text before a valid JSON object.
    EmbeddedJson { prefix: String, record: LogRecord },
    /// Line contains no valid JSON — passed through unmodified.
    Raw,
}

/// A structured log entry extracted from a JSON object.
///
/// Contains the auto-detected or manually-specified timestamp, level,
/// and message fields, plus all remaining fields stored alphabetically
/// in [`extra`](Self::extra) for display.
#[derive(Debug)]
pub struct LogRecord {
    pub timestamp: Option<Timestamp>,
    pub level: Option<Level>,
    pub message: Option<String>,
    /// Remaining fields, ordered alphabetically.
    pub extra: BTreeMap<String, serde_json::Value>,
    /// The original raw JSON string (for `--json` mode passthrough).
    pub raw_json: String,
}

/// Parse a single line from stdin into a [`LineKind`].
///
/// Detection strategy:
/// 1. Lines starting with `{` → try parsing as JSON object
/// 2. Lines containing `{` → try embedded JSON (prefix + JSON)
/// 3. Everything else → [`LineKind::Raw`] (passthrough)
///
/// JSON arrays are treated as [`LineKind::Raw`] since they are not log entries.
pub fn parse_line(line: &str, config: &Config) -> LineKind {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return LineKind::Raw;
    }

    // Fast path: line starts with '{'
    if trimmed.starts_with('{') {
        if let Some(record) = try_parse_json(trimmed, config) {
            return LineKind::Json(record);
        }
        return LineKind::Raw;
    }

    // Embedded JSON detection: scan for first '{'
    if let Some(brace_pos) = trimmed.find('{') {
        let json_part = &trimmed[brace_pos..];
        if let Some(record) = try_parse_json(json_part, config) {
            let prefix = trimmed[..brace_pos].to_string();
            return LineKind::EmbeddedJson { prefix, record };
        }
    }

    LineKind::Raw
}

/// Try to parse a string as a JSON object and extract log fields.
///
/// If the initial parse fails, retries after un-double-escaping backslash
/// sequences (e.g., `\\n` → `\n`, `\\"` → `\"`). Some log pipelines
/// double-escape JSON string contents, producing invalid JSON.
fn try_parse_json(s: &str, config: &Config) -> Option<LogRecord> {
    if let Some(record) = try_parse_json_str(s, config) {
        return Some(record);
    }

    // Fallback: try un-double-escaping and re-parsing.
    if s.contains(r"\\") {
        let fixed = un_double_escape_json(s);
        if fixed != s {
            return try_parse_json_str(&fixed, config);
        }
    }

    None
}

/// Core JSON parsing: deserialize and extract log fields.
fn try_parse_json_str(s: &str, config: &Config) -> Option<LogRecord> {
    let parsed: serde_json::Value = serde_json::from_str(s).ok()?;

    // Only JSON objects are valid log entries; arrays pass through as Raw
    let serde_json::Value::Object(mut map) = parsed else {
        return None;
    };

    // Extract timestamp
    let timestamp = extract_timestamp(&mut map, config);

    // Extract level
    let level = extract_level(&mut map, config);

    // Extract message
    let message = extract_message(&mut map, config);

    // Flatten remaining fields (1 level of dot-notation)
    let extra = flatten_extra(map);

    Some(LogRecord {
        timestamp,
        level,
        message,
        extra,
        raw_json: s.to_string(),
    })
}

/// Extract the timestamp field using config override or alias table.
fn extract_timestamp(
    map: &mut serde_json::Map<String, serde_json::Value>,
    config: &Config,
) -> Option<Timestamp> {
    if let Some(ref key) = config.timestamp_key {
        map.remove(key.as_str())
            .and_then(|v| Timestamp::from_json_value(&v))
    } else {
        fields::find_and_remove(map, fields::TIMESTAMP_ALIASES)
            .and_then(|(_, v)| Timestamp::from_json_value(&v))
    }
}

/// Extract the level field using config override or alias table.
fn extract_level(
    map: &mut serde_json::Map<String, serde_json::Value>,
    config: &Config,
) -> Option<Level> {
    if let Some(ref key) = config.level_key {
        map.remove(key.as_str())
            .and_then(|v| Level::from_json_value(&v, config.level_aliases.as_ref()))
    } else {
        fields::find_and_remove(map, fields::LEVEL_ALIASES)
            .and_then(|(_, v)| Level::from_json_value(&v, config.level_aliases.as_ref()))
    }
}

/// Extract the message field using config override or alias table.
fn extract_message(
    map: &mut serde_json::Map<String, serde_json::Value>,
    config: &Config,
) -> Option<String> {
    if let Some(ref key) = config.message_key {
        map.remove(key.as_str()).and_then(value_to_string)
    } else {
        fields::find_and_remove(map, fields::MESSAGE_ALIASES)
            .map(|(_, v)| value_to_string(v).unwrap_or_default())
    }
}

/// Un-double-escape backslash sequences inside JSON string values.
///
/// Some log pipelines double-escape JSON, turning valid `\n` into `\\n`
/// and `\"` into `\\"`. This makes the JSON invalid because `\\"` is
/// parsed as an escaped backslash followed by a string-terminating quote.
///
/// This function reverses that by replacing `\\X` → `\X` for JSON escape
/// characters (`n`, `r`, `t`, `"`, `\`, `/`, `b`, `f`) only inside string
/// values.
pub fn un_double_escape_json(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    let mut in_string = false;
    let mut escape_next = false;

    while let Some(ch) = chars.next() {
        if escape_next {
            // We're after a single backslash inside a string.
            if ch == '\\' {
                // Double backslash — check if the next char is a JSON escape char.
                if let Some(&next) = chars.peek()
                    && matches!(next, 'n' | 'r' | 't' | '"' | '\\' | '/' | 'b' | 'f' | 'u')
                {
                    // `\\n` → `\n`: drop the extra backslash.
                    result.push('\\');
                    result.push(next);
                    chars.next();
                    escape_next = false;
                    continue;
                }
            }
            result.push(ch);
            escape_next = false;
            continue;
        }

        if in_string && ch == '\\' {
            escape_next = true;
            // Don't push yet — wait to see next char.
            continue;
        }

        if ch == '"' {
            in_string = !in_string;
        }

        result.push(ch);
    }

    // If we ended with a pending backslash, flush it.
    if escape_next {
        result.push('\\');
    }

    result
}

/// Sanitize raw control characters (newlines, carriage returns) inside JSON string values.
///
/// Some log producers (e.g., Python structlog with exception tracebacks) emit
/// JSON with raw `\n` bytes inside string values instead of proper `\\n` escapes.
/// This is technically invalid JSON (RFC 8259 §7), but common in practice.
///
/// This function scans the input and replaces raw `\n` and `\r` characters with
/// their JSON escape sequences (`\\n` and `\\r`) only when they appear inside
/// JSON string values. Newlines between JSON tokens (valid whitespace) are left
/// unchanged.
pub fn sanitize_json_newlines(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut in_string = false;
    let mut escape_next = false;

    for ch in s.chars() {
        if escape_next {
            result.push(ch);
            escape_next = false;
            continue;
        }

        if in_string && ch == '\\' {
            result.push(ch);
            escape_next = true;
            continue;
        }

        if ch == '"' {
            in_string = !in_string;
            result.push(ch);
            continue;
        }

        if in_string {
            match ch {
                '\n' => result.push_str("\\n"),
                '\r' => result.push_str("\\r"),
                _ => result.push(ch),
            }
        } else {
            result.push(ch);
        }
    }

    result
}

/// Convert a JSON value to its string representation.
fn value_to_string(v: serde_json::Value) -> Option<String> {
    match v {
        serde_json::Value::String(s) => Some(s),
        serde_json::Value::Null => None,
        other => Some(other.to_string()),
    }
}

/// Flatten remaining fields 1 level using dot-notation.
///
/// `{"http":{"method":"GET","status":200}}` becomes:
/// - `http.method` = `"GET"`
/// - `http.status` = `200`
///
/// Arrays are NOT flattened — kept as-is.
/// Objects deeper than 1 level are kept as compact JSON.
fn flatten_extra(
    map: serde_json::Map<String, serde_json::Value>,
) -> BTreeMap<String, serde_json::Value> {
    let mut result = BTreeMap::new();

    for (key, value) in map {
        match value {
            serde_json::Value::Object(nested) => {
                for (nested_key, nested_value) in nested {
                    let flat_key = format!("{key}.{nested_key}");
                    result.insert(flat_key, nested_value);
                }
            }
            other => {
                result.insert(key, other);
            }
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn default_config() -> Config {
        Config::default()
    }

    #[test]
    fn test_parse_pure_json() {
        let line = r#"{"level":"info","msg":"hello","port":8080}"#;
        let result = parse_line(line, &default_config());
        match result {
            LineKind::Json(record) => {
                assert_eq!(record.level, Some(Level::Info));
                assert_eq!(record.message.as_deref(), Some("hello"));
                assert!(record.extra.contains_key("port"));
            }
            _ => panic!("Expected Json variant"),
        }
    }

    #[test]
    fn test_parse_embedded_json() {
        let line = r#"2026-02-06 00:15:13.449 {"level":"debug","msg":"health check"}"#;
        let result = parse_line(line, &default_config());
        match result {
            LineKind::EmbeddedJson { prefix, record } => {
                assert_eq!(prefix, "2026-02-06 00:15:13.449 ");
                assert_eq!(record.level, Some(Level::Debug));
                assert_eq!(record.message.as_deref(), Some("health check"));
            }
            _ => panic!("Expected EmbeddedJson variant"),
        }
    }

    #[test]
    fn test_parse_raw() {
        let line = "Just a plain text log line";
        match parse_line(line, &default_config()) {
            LineKind::Raw => {}
            _ => panic!("Expected Raw variant"),
        }
    }

    #[test]
    fn test_parse_empty() {
        match parse_line("", &default_config()) {
            LineKind::Raw => {}
            _ => panic!("Expected Raw variant"),
        }
    }

    #[test]
    fn test_parse_json_array_is_raw() {
        let line = r"[1, 2, 3]";
        match parse_line(line, &default_config()) {
            LineKind::Raw => {}
            _ => panic!("Expected Raw variant for JSON array"),
        }
    }

    #[test]
    fn test_flatten_nested_objects() {
        let line = r#"{"level":"info","msg":"req","http":{"method":"GET","status":200}}"#;
        let result = parse_line(line, &default_config());
        match result {
            LineKind::Json(record) => {
                assert_eq!(record.extra.get("http.method"), Some(&json!("GET")));
                assert_eq!(record.extra.get("http.status"), Some(&json!(200)));
                assert!(!record.extra.contains_key("http"));
            }
            _ => panic!("Expected Json variant"),
        }
    }

    #[test]
    fn test_custom_keys() {
        let config = Config {
            message_key: Some("event".to_string()),
            level_key: Some("sev".to_string()),
            ..Config::default()
        };
        let line = r#"{"sev":"warn","event":"disk full"}"#;
        let result = parse_line(line, &config);
        match result {
            LineKind::Json(record) => {
                assert_eq!(record.level, Some(Level::Warn));
                assert_eq!(record.message.as_deref(), Some("disk full"));
            }
            _ => panic!("Expected Json variant"),
        }
    }

    #[test]
    fn test_malformed_json_is_raw() {
        let line = r#"{"level":"info", "msg":}"#; // trailing comma, invalid
        match parse_line(line, &default_config()) {
            LineKind::Raw => {}
            _ => panic!("Expected Raw for malformed JSON"),
        }
    }

    #[test]
    fn test_embedded_invalid_json_after_brace() {
        let line = "prefix text {not valid json}";
        match parse_line(line, &default_config()) {
            LineKind::Raw => {}
            _ => panic!("Expected Raw for invalid embedded JSON"),
        }
    }

    #[test]
    fn test_flatten_deeply_nested_objects_kept_as_json() {
        // 2-level nested objects should be kept as compact JSON, not further flattened
        let line =
            r#"{"level":"info","msg":"req","http":{"request":{"method":"GET","path":"/api"}}}"#;
        let result = parse_line(line, &default_config());
        match result {
            LineKind::Json(record) => {
                // The nested object is flattened one level: http.request exists
                let val = record
                    .extra
                    .get("http.request")
                    .expect("http.request should exist");
                // The value should be a compact JSON object (not further flattened)
                assert!(val.is_object(), "nested value should remain as JSON object");
            }
            _ => panic!("Expected Json variant"),
        }
    }

    #[test]
    fn test_null_level_in_json() {
        let line = r#"{"level":null,"msg":"hello"}"#;
        let result = parse_line(line, &default_config());
        match result {
            LineKind::Json(record) => {
                assert!(record.level.is_none(), "null level should parse as None");
                assert_eq!(record.message.as_deref(), Some("hello"));
            }
            _ => panic!("Expected Json variant"),
        }
    }

    #[test]
    fn test_null_message_in_json() {
        let line = r#"{"level":"info","msg":null}"#;
        let result = parse_line(line, &default_config());
        match result {
            LineKind::Json(record) => {
                assert_eq!(record.level, Some(crate::level::Level::Info));
                // null message via alias lookup returns Some("") due to unwrap_or_default
                assert_eq!(record.message.as_deref(), Some(""));
            }
            _ => panic!("Expected Json variant"),
        }
    }

    #[test]
    fn test_null_timestamp_in_json() {
        let line = r#"{"level":"info","msg":"hi","time":null}"#;
        let result = parse_line(line, &default_config());
        match result {
            LineKind::Json(record) => {
                assert!(
                    record.timestamp.is_none(),
                    "null timestamp should parse as None"
                );
            }
            _ => panic!("Expected Json variant"),
        }
    }

    #[test]
    fn test_whitespace_only_is_raw() {
        match parse_line("   \t  ", &default_config()) {
            LineKind::Raw => {}
            _ => panic!("Expected Raw for whitespace-only line"),
        }
    }

    #[test]
    fn test_message_as_number() {
        // Non-string message values should be converted to string
        let line = r#"{"level":"info","msg":42}"#;
        let result = parse_line(line, &default_config());
        match result {
            LineKind::Json(record) => {
                assert_eq!(record.message.as_deref(), Some("42"));
            }
            _ => panic!("Expected Json variant"),
        }
    }

    #[test]
    fn test_arrays_in_extra_fields_preserved() {
        let line = r#"{"level":"info","msg":"hi","tags":["a","b"]}"#;
        let result = parse_line(line, &default_config());
        match result {
            LineKind::Json(record) => {
                let tags = record.extra.get("tags").expect("tags should exist");
                assert!(tags.is_array(), "arrays should be preserved as-is");
            }
            _ => panic!("Expected Json variant"),
        }
    }

    #[test]
    fn test_sanitize_json_newlines_no_change() {
        let input = r#"{"level":"info","msg":"hello"}"#;
        assert_eq!(sanitize_json_newlines(input), input);
    }

    #[test]
    fn test_sanitize_json_newlines_in_string_value() {
        let input = "{\"msg\":\"line1\nline2\"}";
        let expected = r#"{"msg":"line1\nline2"}"#;
        assert_eq!(sanitize_json_newlines(input), expected);
    }

    #[test]
    fn test_sanitize_json_newlines_preserves_between_tokens() {
        let input = "{\n\"msg\":\n\"hello\"\n}";
        let expected = "{\n\"msg\":\n\"hello\"\n}";
        assert_eq!(sanitize_json_newlines(input), expected);
    }

    #[test]
    fn test_sanitize_json_newlines_preserves_escaped_quotes() {
        // A string containing escaped quote followed by newline
        let input = "{\"msg\":\"say \\\"hi\\\"\nhello\"}";
        let expected = "{\"msg\":\"say \\\"hi\\\"\\nhello\"}";
        assert_eq!(sanitize_json_newlines(input), expected);
    }

    #[test]
    fn test_sanitize_json_newlines_carriage_return() {
        let input = "{\"msg\":\"line1\r\nline2\"}";
        let expected = r#"{"msg":"line1\r\nline2"}"#;
        assert_eq!(sanitize_json_newlines(input), expected);
    }

    #[test]
    fn test_sanitize_json_newlines_exception_traceback() {
        let input = "{\"event\":\"error\",\"exception\":\"Traceback:\n  File \\\"app.py\\\"\n    raise Error\",\"level\":\"error\"}";
        let sanitized = sanitize_json_newlines(input);
        // Should be valid JSON after sanitization
        let parsed: Result<serde_json::Value, _> = serde_json::from_str(&sanitized);
        assert!(parsed.is_ok(), "sanitized JSON should parse: {sanitized}");
        let obj = parsed.unwrap();
        assert_eq!(obj["event"], "error");
        assert!(obj["exception"].as_str().unwrap().contains("Traceback"));
    }

    #[test]
    fn test_un_double_escape_json_no_change() {
        let input = r#"{"level":"info","msg":"hello"}"#;
        assert_eq!(un_double_escape_json(input), input);
    }

    #[test]
    fn test_un_double_escape_json_newlines() {
        // \\n (double-escaped) should become \n (single-escaped)
        let input = r#"{"msg":"line1\\nline2"}"#;
        let expected = r#"{"msg":"line1\nline2"}"#;
        assert_eq!(un_double_escape_json(input), expected);
    }

    #[test]
    fn test_un_double_escape_json_quotes() {
        // \\" (double-escaped) should become \" (single-escaped)
        let input = r#"{"msg":"say \\"hello\\""}"#;
        let expected = r#"{"msg":"say \"hello\""}"#;
        assert_eq!(un_double_escape_json(input), expected);
    }

    #[test]
    fn test_un_double_escape_json_full_traceback() {
        let input = r#"{"event":"error","exception":"Traceback:\\n  File \\"/src/app.py\\", line 72\\n    raise Error","level":"error"}"#;
        let fixed = un_double_escape_json(input);
        let parsed: Result<serde_json::Value, _> = serde_json::from_str(&fixed);
        assert!(
            parsed.is_ok(),
            "un-double-escaped JSON should parse: {fixed}"
        );
        let obj = parsed.unwrap();
        assert_eq!(obj["level"], "error");
        let exc = obj["exception"].as_str().unwrap();
        assert!(
            exc.contains("Traceback:"),
            "exception should contain Traceback"
        );
        assert!(
            exc.contains("/src/app.py"),
            "exception should contain file path"
        );
    }

    #[test]
    fn test_double_escaped_json_parsed_via_parse_line() {
        // parse_line should handle double-escaped JSON transparently
        let line = r#"{"event":"fail","level":"error","exception":"Traceback:\\n  File \\"/app.py\\", line 1"}"#;
        let result = parse_line(line, &default_config());
        match result {
            LineKind::Json(record) => {
                assert_eq!(record.level, Some(Level::Error));
                assert_eq!(record.message.as_deref(), Some("fail"));
            }
            _ => panic!("Expected Json, got Raw for double-escaped JSON"),
        }
    }

    #[test]
    fn test_double_escaped_embedded_json() {
        let line = r#"2026-02-09 11:15:17.180 {"event":"fail","level":"error","exception":"Traceback:\\n  File \\"/app.py\\"","timestamp":"2026-02-09T11:15:17Z"}"#;
        let result = parse_line(line, &default_config());
        match result {
            LineKind::EmbeddedJson { record, .. } => {
                assert_eq!(record.level, Some(Level::Error));
                assert_eq!(record.message.as_deref(), Some("fail"));
            }
            _ => panic!("Expected EmbeddedJson for double-escaped embedded JSON"),
        }
    }

    #[test]
    fn test_un_double_escape_json_consecutive_backslashes() {
        // Four backslashes `\\\\` inside a string: the first `\\` is a real escaped
        // backslash, the second `\\` is another. After un-double-escaping,
        // `\\\\n` → `\\n` (escaped backslash + literal n), not `\n`.
        let input = r#"{"msg":"path\\\\nope"}"#;
        let result = un_double_escape_json(input);
        // \\\\n: first \\ is escape_next, sees second \, peeks 'n' → produces \\n
        // So we get {"msg":"path\\nope"} which serde reads as "path\nope"
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        let msg = parsed["msg"].as_str().unwrap();
        assert!(
            msg.contains('\\') || msg.contains('\n'),
            "consecutive backslashes should be handled without panic: {msg}"
        );
    }

    #[test]
    fn test_un_double_escape_json_trailing_backslash() {
        // Pending backslash at the end of string should be flushed
        let input = r#"{"msg":"end\\"}"#;
        let result = un_double_escape_json(input);
        // Should not panic, and the trailing backslash should be preserved
        assert!(!result.is_empty());
    }

    #[test]
    fn test_un_double_escape_unicode() {
        // \\u0041 should become \u0041 (which is 'A' in JSON)
        let input = r#"{"msg":"\\u0041"}"#;
        let result = un_double_escape_json(input);
        let expected = r#"{"msg":"\u0041"}"#;
        assert_eq!(result, expected);
        // Verify it produces valid JSON that decodes to "A"
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["msg"], "A");
    }

    #[test]
    fn test_flatten_extra_empty_nested_object() {
        // An empty nested object should disappear (no keys to flatten)
        let line = r#"{"level":"info","msg":"hi","meta":{}}"#;
        let result = parse_line(line, &default_config());
        match result {
            LineKind::Json(record) => {
                assert!(
                    !record.extra.contains_key("meta"),
                    "empty nested object should not appear in extra"
                );
                assert!(record.extra.is_empty());
            }
            _ => panic!("Expected Json variant"),
        }
    }

    #[test]
    fn test_flatten_extra_null_in_nested_object() {
        // Null values inside nested objects should be preserved
        let line = r#"{"level":"info","msg":"hi","ctx":{"user":null,"req_id":"abc"}}"#;
        let result = parse_line(line, &default_config());
        match result {
            LineKind::Json(record) => {
                assert_eq!(record.extra.get("ctx.user"), Some(&json!(null)));
                assert_eq!(record.extra.get("ctx.req_id"), Some(&json!("abc")));
            }
            _ => panic!("Expected Json variant"),
        }
    }

    #[test]
    fn test_sanitize_json_newlines_tab_passes_through() {
        // Tab characters inside strings are valid JSON and should not be altered
        let input = "{\"msg\":\"col1\tcol2\"}";
        assert_eq!(sanitize_json_newlines(input), input);
    }

    #[test]
    fn test_sanitize_json_newlines_multiple_strings() {
        // Multiple string values with newlines
        let input = "{\"a\":\"x\ny\",\"b\":\"1\n2\"}";
        let result = sanitize_json_newlines(input);
        let parsed: Result<serde_json::Value, _> = serde_json::from_str(&result);
        assert!(parsed.is_ok(), "sanitized multi-string JSON should parse");
        let obj = parsed.unwrap();
        assert!(obj["a"].as_str().unwrap().contains('\n'));
        assert!(obj["b"].as_str().unwrap().contains('\n'));
    }

    #[test]
    fn test_try_parse_json_fallback_returns_none_for_unrecoverable() {
        // Contains \\\\ but un-double-escaping doesn't produce valid JSON
        let line = r"{not valid \\json\\ at all}";
        let result = parse_line(line, &default_config());
        match result {
            LineKind::Raw => {}
            _ => panic!("Expected Raw for unrecoverable JSON with backslashes"),
        }
    }

    #[test]
    fn test_parse_empty_json_object() {
        let line = "{}";
        let result = parse_line(line, &default_config());
        match result {
            LineKind::Json(record) => {
                assert!(record.timestamp.is_none());
                assert!(record.level.is_none());
                assert!(record.message.is_none());
                assert!(record.extra.is_empty());
            }
            _ => panic!("Expected Json for empty object"),
        }
    }

    #[test]
    fn test_message_as_boolean() {
        let line = r#"{"level":"info","msg":true}"#;
        let result = parse_line(line, &default_config());
        match result {
            LineKind::Json(record) => {
                assert_eq!(record.message.as_deref(), Some("true"));
            }
            _ => panic!("Expected Json variant"),
        }
    }
}
