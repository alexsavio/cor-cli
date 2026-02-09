//! Timestamp parsing and formatting for structured log entries.
//!
//! Supports ISO 8601, RFC 3339, `YYYY-MM-DD HH:MM:SS` strings, and
//! numeric Unix epochs (seconds, milliseconds, nanoseconds) using a
//! magnitude-based heuristic for disambiguation.

use std::fmt;

/// Parsed and normalized timestamp representation.
///
/// Wraps a [`jiff::Timestamp`] for high-precision time handling.
/// The [`format_display`](Self::format_display) method outputs `HH:MM:SS.mmm` in UTC.
#[derive(Debug, Clone)]
pub struct Timestamp {
    /// Normalized timestamp value.
    pub value: jiff::Timestamp,
    /// Original string representation for fallback display.
    #[allow(dead_code)] // Available for fallback/verbose display modes
    pub original: String,
}

impl Timestamp {
    /// Format the timestamp for display using the given strftime-compatible format string.
    pub fn format_with(&self, format: &str) -> String {
        let zdt = self.value.to_zoned(jiff::tz::TimeZone::UTC);
        zdt.strftime(format).to_string()
    }

    /// Format the timestamp using the default format (`YYYY-MM-DDTHH:MM:SS.mmm`).
    pub fn format_display(&self) -> String {
        self.format_with("%Y-%m-%dT%H:%M:%S%.3f")
    }

    /// Parse a timestamp from a [`serde_json::Value`].
    ///
    /// Supports:
    /// - ISO 8601 / RFC 3339 strings
    /// - `YYYY-MM-DD HH:MM:SS` format
    /// - Unix epoch seconds (integer or float)
    /// - Unix epoch milliseconds (integer)
    /// - Unix epoch nanoseconds (integer)
    pub fn from_json_value(value: &serde_json::Value) -> Option<Self> {
        match value {
            serde_json::Value::String(s) => Self::parse_string(s),
            serde_json::Value::Number(n) => Self::parse_number(n),
            _ => None,
        }
    }

    /// Parse a string timestamp.
    fn parse_string(s: &str) -> Option<Self> {
        let original = s.to_string();

        // Try ISO 8601 / RFC 3339; jiff handles these natively
        if let Ok(ts) = s.parse::<jiff::Timestamp>() {
            return Some(Self {
                value: ts,
                original,
            });
        }

        // Try YYYY-MM-DD HH:MM:SS (no timezone → assume UTC)
        if let Ok(dt) = jiff::civil::DateTime::strptime("%Y-%m-%d %H:%M:%S", s)
            && let Ok(ts) = dt.to_zoned(jiff::tz::TimeZone::UTC)
        {
            return Some(Self {
                value: ts.timestamp(),
                original,
            });
        }

        // Try YYYY-MM-DD HH:MM:SS.fff
        if let Ok(dt) = jiff::civil::DateTime::strptime("%Y-%m-%d %H:%M:%S%.f", s)
            && let Ok(ts) = dt.to_zoned(jiff::tz::TimeZone::UTC)
        {
            return Some(Self {
                value: ts.timestamp(),
                original,
            });
        }

        None
    }

    /// Parse a numeric timestamp using the heuristic:
    /// - Value < 1e12 → seconds
    /// - Value < 1e15 → milliseconds
    /// - Value ≥ 1e15 → nanoseconds
    fn parse_number(n: &serde_json::Number) -> Option<Self> {
        if let Some(i) = n.as_i64() {
            Self::from_epoch_integer(i, n.to_string())
        } else if let Some(f) = n.as_f64() {
            Self::from_epoch_float(f, n.to_string())
        } else {
            None
        }
    }

    fn from_epoch_integer(value: i64, original: String) -> Option<Self> {
        let ts = if value < 1_000_000_000_000 {
            // seconds
            jiff::Timestamp::from_second(value).ok()?
        } else if value < 1_000_000_000_000_000 {
            // milliseconds
            jiff::Timestamp::from_millisecond(value).ok()?
        } else {
            // nanoseconds
            jiff::Timestamp::from_nanosecond(i128::from(value)).ok()?
        };
        Some(Self {
            value: ts,
            original,
        })
    }

    fn from_epoch_float(value: f64, original: String) -> Option<Self> {
        if value < 1e12 {
            // seconds with fractional part
            #[allow(clippy::cast_possible_truncation)]
            let secs = value.trunc() as i64;
            #[allow(clippy::cast_possible_truncation)]
            let nanos = ((value.fract()) * 1_000_000_000.0) as i32;
            let ts = jiff::Timestamp::new(secs, nanos).ok()?;
            Some(Self {
                value: ts,
                original,
            })
        } else {
            // milliseconds as float
            #[allow(clippy::cast_possible_truncation)]
            let ms = value as i64;
            let ts = jiff::Timestamp::from_millisecond(ms).ok()?;
            Some(Self {
                value: ts,
                original,
            })
        }
    }
}

impl fmt::Display for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.format_display())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_iso8601() {
        let val = json!("2026-01-15T10:30:00.123Z");
        let ts = Timestamp::from_json_value(&val).unwrap();
        assert_eq!(ts.format_display(), "2026-01-15T10:30:00.123");
    }

    #[test]
    fn test_parse_iso8601_with_offset() {
        let val = json!("2026-01-15T12:30:00.000+02:00");
        let ts = Timestamp::from_json_value(&val).unwrap();
        // 12:30 +02:00 = 10:30 UTC
        assert_eq!(ts.format_display(), "2026-01-15T10:30:00.000");
    }

    #[test]
    fn test_parse_epoch_seconds_integer() {
        // 2026-01-15 10:30:00 UTC = 1768473000
        let val = json!(1_768_473_000);
        let ts = Timestamp::from_json_value(&val).unwrap();
        assert_eq!(ts.format_display(), "2026-01-15T10:30:00.000");
    }

    #[test]
    fn test_parse_epoch_seconds_float() {
        let val = json!(1_768_473_000.123);
        let ts = Timestamp::from_json_value(&val).unwrap();
        assert!(ts.format_display().starts_with("2026-01-15T10:30:00."));
    }

    #[test]
    fn test_parse_epoch_milliseconds() {
        let val = json!(1_768_473_000_123_i64);
        let ts = Timestamp::from_json_value(&val).unwrap();
        assert_eq!(ts.format_display(), "2026-01-15T10:30:00.123");
    }

    #[test]
    fn test_parse_epoch_nanoseconds() {
        let val = json!(1_768_473_000_123_000_000_i64);
        let ts = Timestamp::from_json_value(&val).unwrap();
        assert_eq!(ts.format_display(), "2026-01-15T10:30:00.123");
    }

    #[test]
    fn test_parse_datetime_no_tz() {
        let val = json!("2026-01-15 10:30:00");
        let ts = Timestamp::from_json_value(&val).unwrap();
        assert_eq!(ts.format_display(), "2026-01-15T10:30:00.000");
    }

    #[test]
    fn test_parse_invalid() {
        assert!(Timestamp::from_json_value(&json!("not-a-timestamp")).is_none());
        assert!(Timestamp::from_json_value(&json!(true)).is_none());
        assert!(Timestamp::from_json_value(&json!(null)).is_none());
    }

    #[test]
    fn test_format_with_custom() {
        let val = json!("2026-01-15T10:30:00.123Z");
        let ts = Timestamp::from_json_value(&val).unwrap();
        assert_eq!(ts.format_with("%H:%M:%S"), "10:30:00");
    }

    #[test]
    fn test_format_with_full_datetime() {
        let val = json!("2026-01-15T10:30:00.123Z");
        let ts = Timestamp::from_json_value(&val).unwrap();
        assert_eq!(ts.format_with("%Y-%m-%d %H:%M:%S"), "2026-01-15 10:30:00");
    }

    #[test]
    fn test_format_display_uses_default_format() {
        let val = json!("2026-01-15T10:30:00.123Z");
        let ts = Timestamp::from_json_value(&val).unwrap();
        // format_display() should match format_with() using the default format
        assert_eq!(ts.format_display(), ts.format_with("%Y-%m-%dT%H:%M:%S%.3f"));
    }

    #[test]
    fn test_display_trait() {
        let val = json!("2026-01-15T10:30:00.123Z");
        let ts = Timestamp::from_json_value(&val).unwrap();
        // Display trait uses format_display()
        assert_eq!(format!("{ts}"), ts.format_display());
    }

    #[test]
    fn test_epoch_zero() {
        let val = json!(0);
        let ts = Timestamp::from_json_value(&val).unwrap();
        assert_eq!(ts.format_display(), "1970-01-01T00:00:00.000");
    }

    #[test]
    fn test_parse_datetime_with_fractional_seconds() {
        let val = json!("2026-01-15 10:30:00.456");
        let ts = Timestamp::from_json_value(&val).unwrap();
        assert!(ts.format_display().starts_with("2026-01-15T10:30:00."));
    }

    #[test]
    fn test_epoch_boundary_seconds_to_milliseconds() {
        // Exactly 1_000_000_000_000 should be treated as milliseconds, not seconds
        let val = json!(1_000_000_000_000_i64);
        let ts = Timestamp::from_json_value(&val).unwrap();
        // 1e12 ms = 2001-09-09T01:46:40Z (milliseconds path)
        assert!(ts.format_display().starts_with("2001-09-09"));

        // One below: 999_999_999_999 would be treated as seconds, but that's
        // ~31688 years which overflows jiff's representable range → None
        let val = json!(999_999_999_999_i64);
        assert!(
            Timestamp::from_json_value(&val).is_none(),
            "seconds value near 1e12 exceeds jiff timestamp range"
        );

        // A realistic seconds value still works
        let val = json!(1_700_000_000_i64);
        let ts = Timestamp::from_json_value(&val).unwrap();
        assert!(ts.format_display().starts_with("2023-"));
    }

    #[test]
    fn test_epoch_boundary_milliseconds_to_nanoseconds() {
        // Exactly 1_000_000_000_000_000 should be treated as nanoseconds
        let val = json!(1_000_000_000_000_000_i64);
        let ts = Timestamp::from_json_value(&val).unwrap();
        // 1e15 ns = 1e6 seconds ≈ 1970-01-12
        assert!(ts.format_display().starts_with("1970-01-12"));

        // One below: 999_999_999_999_999 would be treated as milliseconds, but
        // that's ~31688 years which overflows jiff's representable range → None
        let val = json!(999_999_999_999_999_i64);
        assert!(
            Timestamp::from_json_value(&val).is_none(),
            "milliseconds value near 1e15 exceeds jiff timestamp range"
        );

        // A realistic nanoseconds value works
        let val = json!(1_700_000_000_000_000_000_i64);
        let ts = Timestamp::from_json_value(&val).unwrap();
        assert!(ts.format_display().starts_with("2023-"));
    }

    #[test]
    fn test_negative_epoch_seconds() {
        // Before Unix epoch: 1969-12-31T23:59:59Z
        let val = json!(-1);
        let ts = Timestamp::from_json_value(&val).unwrap();
        assert!(ts.format_display().starts_with("1969-12-31"));
    }

    #[test]
    fn test_epoch_float_boundary() {
        // Float value at exactly 1e12 should take the milliseconds branch
        let val = json!(1_000_000_000_000.0_f64);
        let ts = Timestamp::from_json_value(&val).unwrap();
        // 1e12 ms ≈ 2001-09-09
        assert!(ts.format_display().starts_with("2001-09-09"));

        // Float value below 1e12 but too large for seconds → overflows jiff range
        let val = json!(999_999_999_999.5_f64);
        assert!(
            Timestamp::from_json_value(&val).is_none(),
            "float seconds near 1e12 exceeds jiff timestamp range"
        );

        // A realistic float seconds value works (fractional seconds preserved)
        let val = json!(1_700_000_000.5_f64);
        let ts = Timestamp::from_json_value(&val).unwrap();
        assert!(ts.format_display().starts_with("2023-"));
    }
}
