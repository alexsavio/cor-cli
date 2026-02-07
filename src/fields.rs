//! Canonical field alias tables for auto-detecting common log fields.
//!
//! Aliases are ordered by frequency of use across frameworks (logrus, zap, slog,
//! pino, bunyan, structlog). First match wins during field extraction.

/// Known aliases for timestamp fields.
pub const TIMESTAMP_ALIASES: &[&str] = &[
    "time",
    "ts",
    "timestamp",
    "@timestamp",
    "datetime",
    "date",
    "t",
    "logged_at",
    "created_at",
];

/// Known aliases for level/severity fields.
pub const LEVEL_ALIASES: &[&str] = &[
    "level",
    "severity",
    "loglevel",
    "log_level",
    "lvl",
    "priority",
    "log.level",
];

/// Known aliases for message fields.
pub const MESSAGE_ALIASES: &[&str] = &[
    "msg",
    "message",
    "text",
    "log",
    "body",
    "event",
    "short_message",
];

/// Known aliases for logger name fields.
#[allow(dead_code)] // Ready for use when logger field extraction is added
pub const LOGGER_ALIASES: &[&str] = &["logger", "name", "logger_name", "component", "module"];

/// Known aliases for caller/source fields.
#[allow(dead_code)] // Ready for use when caller field extraction is added
pub const CALLER_ALIASES: &[&str] = &[
    "caller", "source", "src", "location", "file", "func", "function",
];

/// Known aliases for error fields.
#[allow(dead_code)] // Ready for use when error field extraction is added
pub const ERROR_ALIASES: &[&str] = &[
    "error",
    "err",
    "exception",
    "exc_info",
    "stack_trace",
    "stacktrace",
    "stack",
];

/// Look up the first matching alias key in a JSON object.
///
/// Returns the key name and removes it from the map if found.
pub fn find_and_remove(
    map: &mut serde_json::Map<String, serde_json::Value>,
    aliases: &[&str],
) -> Option<(String, serde_json::Value)> {
    for &alias in aliases {
        if let Some(val) = map.remove(alias) {
            return Some((alias.to_string(), val));
        }
    }
    None
}

/// Look up the first matching alias key in a JSON object without removing it.
#[allow(dead_code)] // Public API for read-only alias lookup
pub fn find_key<'a>(
    map: &'a serde_json::Map<String, serde_json::Value>,
    aliases: &[&'a str],
) -> Option<&'a str> {
    aliases
        .iter()
        .find(|&&alias| map.contains_key(alias))
        .copied()
        .map(|v| v as _)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_find_and_remove_first_match() {
        let mut map = serde_json::Map::new();
        map.insert("ts".to_string(), json!(1_234_567_890));
        map.insert("time".to_string(), json!("2026-01-01T00:00:00Z"));

        // "time" is first in TIMESTAMP_ALIASES, so it wins
        let result = find_and_remove(&mut map, TIMESTAMP_ALIASES);
        assert!(result.is_some());
        let (key, _val) = result.unwrap();
        assert_eq!(key, "time");
        // "time" removed from map
        assert!(!map.contains_key("time"));
        // "ts" still present
        assert!(map.contains_key("ts"));
    }

    #[test]
    fn test_find_and_remove_none() {
        let mut map = serde_json::Map::new();
        map.insert("foo".to_string(), json!("bar"));

        let result = find_and_remove(&mut map, TIMESTAMP_ALIASES);
        assert!(result.is_none());
    }

    #[test]
    fn test_find_key() {
        let mut map = serde_json::Map::new();
        map.insert("msg".to_string(), json!("hello"));
        assert_eq!(find_key(&map, MESSAGE_ALIASES), Some("msg"));

        map.clear();
        map.insert("event".to_string(), json!("hello"));
        assert_eq!(find_key(&map, MESSAGE_ALIASES), Some("event"));

        map.clear();
        map.insert("unknown".to_string(), json!("hello"));
        assert_eq!(find_key(&map, MESSAGE_ALIASES), None);
    }

    #[test]
    fn test_find_and_remove_empty_aliases() {
        let mut map = serde_json::Map::new();
        map.insert("foo".to_string(), json!("bar"));
        let result = find_and_remove(&mut map, &[]);
        assert!(result.is_none());
        // Map unchanged
        assert!(map.contains_key("foo"));
    }

    #[test]
    fn test_find_key_empty_map() {
        let map = serde_json::Map::new();
        assert_eq!(find_key(&map, TIMESTAMP_ALIASES), None);
    }

    #[test]
    fn test_find_and_remove_returns_value() {
        let mut map = serde_json::Map::new();
        map.insert("severity".to_string(), json!("error"));
        let result = find_and_remove(&mut map, LEVEL_ALIASES);
        let (key, val) = result.unwrap();
        assert_eq!(key, "severity");
        assert_eq!(val, json!("error"));
        assert!(map.is_empty());
    }
}
