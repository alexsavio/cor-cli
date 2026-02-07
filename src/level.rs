//! Log level representation with parsing, display, and colorization.
//!
//! Supports both string-based levels (e.g., `"info"`, `"warn"`) and numeric
//! levels used by frameworks like bunyan and pino (e.g., 30 = info, 40 = warn).
//! Includes aliases from major logging frameworks for case-insensitive matching.

use std::fmt;

use owo_colors::Style;

/// Canonical log level enumeration.
///
/// Ordered by severity (ascending) for `>=` filtering via [`Ord`].
/// Each variant has a numeric discriminant matching the bunyan/pino convention:
/// - [`Trace`](Self::Trace) = 10
/// - [`Debug`](Self::Debug) = 20
/// - [`Info`](Self::Info) = 30
/// - [`Warn`](Self::Warn) = 40
/// - [`Error`](Self::Error) = 50
/// - [`Fatal`](Self::Fatal) = 60
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Level {
    Trace = 10,
    Debug = 20,
    Info = 30,
    Warn = 40,
    Error = 50,
    Fatal = 60,
}

impl Level {
    /// 5-character display badge for the level, right-justified (e.g., `" INFO"`, `"ERROR"`).
    #[allow(clippy::trivially_copy_pass_by_ref)] // &self required since OwoColorize has conflicting trait methods
    pub const fn badge(&self) -> &'static str {
        match self {
            Self::Trace => "TRACE",
            Self::Debug => "DEBUG",
            Self::Info => " INFO",
            Self::Warn => " WARN",
            Self::Error => "ERROR",
            Self::Fatal => "FATAL",
        }
    }

    /// The blank badge (5 spaces) used when no level is recognized.
    pub const fn blank_badge() -> &'static str {
        "     "
    }

    /// Returns the [`Style`] for this level's badge when colors are enabled.
    ///
    /// Color scheme follows fblog convention:
    /// - Trace: cyan bold
    /// - Debug: blue bold
    /// - Info: green bold
    /// - Warn: yellow bold
    /// - Error: red bold
    /// - Fatal: magenta bold
    #[allow(clippy::trivially_copy_pass_by_ref)] // &self required since OwoColorize has conflicting trait methods
    pub const fn style(&self) -> Style {
        match self {
            Self::Trace => Style::new().cyan().bold(),
            Self::Debug => Style::new().blue().bold(),
            Self::Info => Style::new().green().bold(),
            Self::Warn => Style::new().yellow().bold(),
            Self::Error => Style::new().red().bold(),
            Self::Fatal => Style::new().magenta().bold(),
        }
    }

    /// Parse a string into a [`Level`], case-insensitive.
    ///
    /// Returns `None` for unrecognized strings.
    pub fn from_str_loose(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "trace" | "trc" => Some(Self::Trace),
            "debug" | "dbg" => Some(Self::Debug),
            "info" | "inf" | "information" => Some(Self::Info),
            "warn" | "warning" | "wrn" => Some(Self::Warn),
            "error" | "err" | "fatal_error" => Some(Self::Error),
            "fatal" | "critical" | "crit" | "panic" | "emerg" | "emergency" => Some(Self::Fatal),
            _ => None,
        }
    }

    /// Parse a numeric value into a [`Level`] using nearest-match rounding.
    ///
    /// Uses bunyan/pino numeric convention:
    /// - 10 = trace, 20 = debug, 30 = info, 40 = warn, 50 = error, 60 = fatal
    ///
    /// Values between thresholds round to the nearest lower level.
    pub const fn from_numeric(n: i64) -> Self {
        match n {
            ..=14 => Self::Trace,
            15..=24 => Self::Debug,
            25..=34 => Self::Info,
            35..=44 => Self::Warn,
            45..=54 => Self::Error,
            55.. => Self::Fatal,
        }
    }

    /// Parse a level from a [`serde_json::Value`].
    ///
    /// Handles both string and numeric representations.
    pub fn from_json_value(
        value: &serde_json::Value,
        custom_aliases: Option<&std::collections::HashMap<String, Self>>,
    ) -> Option<Self> {
        match value {
            serde_json::Value::String(s) => {
                // Check custom aliases first
                if let Some(aliases) = custom_aliases
                    && let Some(level) = aliases.get(&s.to_lowercase())
                {
                    return Some(*level);
                }
                Self::from_str_loose(s)
            }
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Some(Self::from_numeric(i))
                } else {
                    #[allow(clippy::cast_possible_truncation)]
                    n.as_f64().map(|f| Self::from_numeric(f as i64))
                }
            }
            _ => None,
        }
    }
}

impl fmt::Display for Level {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.badge())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_str_loose_basic() {
        assert_eq!(Level::from_str_loose("info"), Some(Level::Info));
        assert_eq!(Level::from_str_loose("INFO"), Some(Level::Info));
        assert_eq!(Level::from_str_loose("Info"), Some(Level::Info));
        assert_eq!(Level::from_str_loose("warn"), Some(Level::Warn));
        assert_eq!(Level::from_str_loose("WARNING"), Some(Level::Warn));
        assert_eq!(Level::from_str_loose("error"), Some(Level::Error));
        assert_eq!(Level::from_str_loose("debug"), Some(Level::Debug));
        assert_eq!(Level::from_str_loose("trace"), Some(Level::Trace));
        assert_eq!(Level::from_str_loose("fatal"), Some(Level::Fatal));
        assert_eq!(Level::from_str_loose("critical"), Some(Level::Fatal));
        assert_eq!(Level::from_str_loose("panic"), Some(Level::Fatal));
    }

    #[test]
    fn test_from_str_loose_unknown() {
        assert_eq!(Level::from_str_loose("verbose"), None);
        assert_eq!(Level::from_str_loose(""), None);
        assert_eq!(Level::from_str_loose("nonsense"), None);
    }

    #[test]
    fn test_from_numeric() {
        assert_eq!(Level::from_numeric(10), Level::Trace);
        assert_eq!(Level::from_numeric(20), Level::Debug);
        assert_eq!(Level::from_numeric(30), Level::Info);
        assert_eq!(Level::from_numeric(40), Level::Warn);
        assert_eq!(Level::from_numeric(50), Level::Error);
        assert_eq!(Level::from_numeric(60), Level::Fatal);
    }

    #[test]
    fn test_from_numeric_nearest_match() {
        // Between thresholds: rounds to nearest lower level
        assert_eq!(Level::from_numeric(25), Level::Info);
        assert_eq!(Level::from_numeric(35), Level::Warn);
        assert_eq!(Level::from_numeric(45), Level::Error);
        assert_eq!(Level::from_numeric(5), Level::Trace);
        assert_eq!(Level::from_numeric(100), Level::Fatal);
    }

    #[test]
    fn test_level_ordering() {
        assert!(Level::Trace < Level::Debug);
        assert!(Level::Debug < Level::Info);
        assert!(Level::Info < Level::Warn);
        assert!(Level::Warn < Level::Error);
        assert!(Level::Error < Level::Fatal);
    }

    #[test]
    fn test_badge_width() {
        // All badges must be exactly 5 characters for alignment
        for level in [
            Level::Trace,
            Level::Debug,
            Level::Info,
            Level::Warn,
            Level::Error,
            Level::Fatal,
        ] {
            assert_eq!(level.badge().len(), 5, "Badge for {level:?} is not 5 chars");
        }
        assert_eq!(Level::blank_badge().len(), 5);
    }

    #[test]
    fn test_from_json_value_string() {
        let val = serde_json::Value::String("info".to_string());
        assert_eq!(Level::from_json_value(&val, None), Some(Level::Info));
    }

    #[test]
    fn test_from_json_value_number() {
        let val = serde_json::json!(30);
        assert_eq!(Level::from_json_value(&val, None), Some(Level::Info));
    }

    #[test]
    fn test_from_json_value_custom_alias() {
        let mut aliases = std::collections::HashMap::new();
        aliases.insert("verbose".to_string(), Level::Debug);
        let val = serde_json::Value::String("verbose".to_string());
        assert_eq!(
            Level::from_json_value(&val, Some(&aliases)),
            Some(Level::Debug)
        );
    }
}
