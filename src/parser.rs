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
fn try_parse_json(s: &str, config: &Config) -> Option<LogRecord> {
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
}
