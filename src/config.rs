//! Configuration management with TOML file support.
//!
//! Merges settings from three sources (highest precedence first):
//! 1. CLI flags
//! 2. Config file (`~/.config/cor/config.toml` or `$XDG_CONFIG_HOME/cor/config.toml`)
//! 3. Built-in defaults

use std::collections::HashMap;
use std::path::PathBuf;

use serde::Deserialize;

use crate::cli::{Cli, ColorMode};
use crate::error::CorError;
use crate::level::Level;

/// Runtime configuration merged from defaults, config file, and CLI arguments.
///
/// Use [`Config::from_cli`] to build from parsed CLI arguments, or
/// [`Config::default`] for built-in defaults (useful in tests and benchmarks).
#[derive(Debug, Clone)]
pub struct Config {
    /// Color output mode (auto/always/never).
    pub color_mode: ColorMode,
    /// Minimum log level to display; lines below this are suppressed.
    pub min_level: Option<Level>,
    /// Custom JSON key for the message field (overrides alias table).
    pub message_key: Option<String>,
    /// Custom JSON key for the level field (overrides alias table).
    pub level_key: Option<String>,
    /// Custom JSON key for the timestamp field (overrides alias table).
    pub timestamp_key: Option<String>,
    /// Whitelist of extra fields to display (mutually exclusive with `exclude_fields`).
    pub include_fields: Option<Vec<String>>,
    /// Blacklist of extra fields to hide (mutually exclusive with `include_fields`).
    pub exclude_fields: Option<Vec<String>>,
    /// Output raw JSON instead of colorized text (for piping to other tools).
    pub json_output: bool,
    /// Maximum character length for extra field values before truncation. 0 = no limit.
    pub max_field_length: usize,
    /// Timestamp display format string (strftime-compatible).
    pub timestamp_format: String,
    /// Custom level name aliases mapping string → [`Level`].
    pub level_aliases: Option<HashMap<String, Level>>,
    /// Number of blank lines inserted between each log entry. 0 = compact (no gaps).
    pub line_gap: usize,
    /// Minimum width for extra field key alignment (right-justified).
    pub key_min_width: usize,
    /// Custom colors for log level badges (maps level → color name).
    pub level_colors: Option<HashMap<Level, String>>,
    /// Show parse errors for lines that look like JSON but fail to parse.
    pub verbose: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            color_mode: ColorMode::Auto,
            min_level: None,
            message_key: None,
            level_key: None,
            timestamp_key: None,
            include_fields: None,
            exclude_fields: None,
            json_output: false,
            max_field_length: 120,
            timestamp_format: "%Y-%m-%dT%H:%M:%S%.3f".to_string(),
            level_aliases: None,
            line_gap: 1,
            key_min_width: 25,
            level_colors: None,
            verbose: false,
        }
    }
}

impl Config {
    /// Build a [`Config`] from CLI arguments, loading the config file if present.
    ///
    /// Merge precedence: CLI flags > config file > defaults.
    pub fn from_cli(cli: &Cli) -> Result<Self, CorError> {
        // Start with defaults
        let mut config = Self::default();

        // Load config file if it exists
        let config_path = cli.config.clone().unwrap_or_else(Self::default_config_path);

        if config_path.exists() {
            let file_config = FileConfig::load(&config_path)?;
            config.apply_file_config(file_config);
        }

        // CLI overrides (CLI takes precedence over config file)
        config.color_mode = cli.color;

        if let Some(ref level_str) = cli.level {
            config.min_level = Level::from_str_loose(level_str);
        }

        // CLI key overrides replace config file settings
        if let Some(ref key) = cli.message_key {
            config.message_key = Some(key.clone());
        }
        if let Some(ref key) = cli.level_key {
            config.level_key = Some(key.clone());
        }
        if let Some(ref key) = cli.timestamp_key {
            config.timestamp_key = Some(key.clone());
        }
        if let Some(ref fields) = cli.include_fields {
            config.include_fields = Some(fields.clone());
        }
        if let Some(ref fields) = cli.exclude_fields {
            config.exclude_fields = Some(fields.clone());
        }

        config.json_output = cli.json;
        config.verbose = cli.verbose;
        if let Some(max_len) = cli.max_field_length {
            config.max_field_length = max_len;
        }
        if let Some(gap) = cli.line_gap {
            config.line_gap = gap;
        }

        Ok(config)
    }

    /// Default config file path: `$XDG_CONFIG_HOME/cor/config.toml` or `~/.config/cor/config.toml`.
    fn default_config_path() -> PathBuf {
        if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME") {
            PathBuf::from(xdg).join("cor").join("config.toml")
        } else if let Some(home) = std::env::var_os("HOME") {
            PathBuf::from(home)
                .join(".config")
                .join("cor")
                .join("config.toml")
        } else {
            PathBuf::from(".config/cor/config.toml")
        }
    }

    /// Apply settings from a parsed config file.
    fn apply_file_config(&mut self, file: FileConfig) {
        if let Some(color) = file.color {
            self.color_mode = match color.as_str() {
                "always" => ColorMode::Always,
                "never" => ColorMode::Never,
                _ => ColorMode::Auto,
            };
        }

        if let Some(level) = file.level {
            self.min_level = Level::from_str_loose(&level);
        }

        if let Some(format) = file.timestamp_format {
            self.timestamp_format = format;
        }

        if let Some(max_len) = file.max_field_length {
            self.max_field_length = max_len;
        }

        if let Some(gap) = file.line_gap {
            self.line_gap = gap;
        }

        if let Some(width) = file.key_min_width {
            self.key_min_width = width;
        }

        if let Some(keys) = file.keys {
            if let Some(msg) = keys.message {
                self.message_key = Some(msg);
            }
            if let Some(lvl) = keys.level {
                self.level_key = Some(lvl);
            }
            if let Some(ts) = keys.timestamp {
                self.timestamp_key = Some(ts);
            }
        }

        if let Some(levels) = file.levels {
            let mut aliases = HashMap::new();
            for (key, value) in levels {
                if let Some(level) = Level::from_str_loose(&value) {
                    aliases.insert(key.to_lowercase(), level);
                }
            }
            if !aliases.is_empty() {
                self.level_aliases = Some(aliases);
            }
        }

        if let Some(colors) = file.colors {
            let mut level_colors = HashMap::new();
            for (level_str, color) in colors {
                if let Some(level) = Level::from_str_loose(&level_str) {
                    // Validate color name
                    if is_valid_color(&color) {
                        level_colors.insert(level, color.to_lowercase());
                    }
                }
            }
            if !level_colors.is_empty() {
                self.level_colors = Some(level_colors);
            }
        }
    }
}

/// Check if a color name is valid.
fn is_valid_color(color: &str) -> bool {
    matches!(
        color.to_lowercase().as_str(),
        "black"
            | "red"
            | "green"
            | "yellow"
            | "blue"
            | "magenta"
            | "purple"
            | "cyan"
            | "white"
            | "bright_black"
            | "bright_red"
            | "bright_green"
            | "bright_yellow"
            | "bright_blue"
            | "bright_magenta"
            | "bright_cyan"
            | "bright_white"
    )
}

/// Config file structure (TOML deserialization).
#[derive(Debug, Deserialize)]
struct FileConfig {
    color: Option<String>,
    level: Option<String>,
    timestamp_format: Option<String>,
    max_field_length: Option<usize>,
    line_gap: Option<usize>,
    key_min_width: Option<usize>,
    keys: Option<KeysConfig>,
    levels: Option<HashMap<String, String>>,
    colors: Option<HashMap<String, String>>,
}

#[derive(Debug, Deserialize)]
struct KeysConfig {
    message: Option<String>,
    level: Option<String>,
    timestamp: Option<String>,
}

impl FileConfig {
    fn load(path: &PathBuf) -> Result<Self, CorError> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            CorError::Config(format!("cannot read config file {}: {e}", path.display()))
        })?;
        let config: Self = toml::from_str(&content)?;
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.color_mode, ColorMode::Auto);
        assert!(config.min_level.is_none());
        assert!(config.message_key.is_none());
        assert_eq!(config.max_field_length, 120);
        assert_eq!(config.timestamp_format, "%Y-%m-%dT%H:%M:%S%.3f");
        assert!(!config.json_output);
        assert_eq!(config.line_gap, 1);
        assert_eq!(config.key_min_width, 25);
    }

    #[test]
    fn test_file_config_parse() {
        let toml_str = r#"
            color = "always"
            level = "warn"
            timestamp_format = "%H:%M:%S"
            max_field_length = 80
            line_gap = 2

            [keys]
            message = "event"
            level = "severity"
            timestamp = "datetime"

            [levels]
            "verbose" = "debug"
            "critical" = "fatal"
        "#;

        let file_config: FileConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(file_config.color.as_deref(), Some("always"));
        assert_eq!(file_config.level.as_deref(), Some("warn"));
        assert_eq!(file_config.max_field_length, Some(80));
        assert_eq!(file_config.line_gap, Some(2));
        assert!(file_config.keys.is_some());
        assert!(file_config.levels.is_some());
    }

    #[test]
    fn test_apply_file_config() {
        let mut config = Config::default();
        let file_config = FileConfig {
            color: Some("never".to_string()),
            level: Some("error".to_string()),
            timestamp_format: Some("%H:%M:%S".to_string()),
            max_field_length: Some(80),
            line_gap: Some(3),
            key_min_width: Some(30),
            keys: Some(KeysConfig {
                message: Some("event".to_string()),
                level: None,
                timestamp: None,
            }),
            levels: Some({
                let mut m = HashMap::new();
                m.insert("verbose".to_string(), "debug".to_string());
                m
            }),
            colors: None,
        };

        config.apply_file_config(file_config);
        assert_eq!(config.color_mode, ColorMode::Never);
        assert_eq!(config.min_level, Some(Level::Error));
        assert_eq!(config.message_key.as_deref(), Some("event"));
        assert_eq!(config.max_field_length, 80);
        assert_eq!(config.line_gap, 3);
        assert_eq!(config.key_min_width, 30);
        assert!(config.level_aliases.is_some());
    }

    #[test]
    fn test_file_config_load_nonexistent() {
        let path = PathBuf::from("/tmp/cor-test-nonexistent-config.toml");
        let result = FileConfig::load(&path);
        assert!(result.is_err());
        let err = result.unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("cannot read config file"),
            "expected config error, got: {msg}"
        );
    }

    #[test]
    fn test_file_config_load_invalid_toml() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(&path, "this is not valid [[ toml").unwrap();
        let result = FileConfig::load(&path);
        assert!(result.is_err());
        let err = result.unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("config file error"),
            "expected TOML parse error, got: {msg}"
        );
    }

    #[test]
    fn test_apply_file_config_partial() {
        // Only set some fields; others remain as defaults
        let mut config = Config::default();
        let file_config = FileConfig {
            color: None,
            level: None,
            timestamp_format: Some("%H:%M".to_string()),
            max_field_length: None,
            line_gap: None,
            key_min_width: None,
            keys: None,
            levels: None,
            colors: None,
        };
        config.apply_file_config(file_config);
        assert_eq!(config.color_mode, ColorMode::Auto);
        assert!(config.min_level.is_none());
        assert_eq!(config.timestamp_format, "%H:%M");
        assert_eq!(config.max_field_length, 120);
        assert_eq!(config.line_gap, 1);
        assert_eq!(config.key_min_width, 25);
    }

    #[test]
    fn test_apply_file_config_invalid_level_aliases_skipped() {
        // Level aliases mapping to unrecognized level strings should be silently skipped
        let mut config = Config::default();
        let file_config = FileConfig {
            color: None,
            level: None,
            timestamp_format: None,
            max_field_length: None,
            line_gap: None,
            key_min_width: None,
            keys: None,
            levels: Some({
                let mut m = HashMap::new();
                m.insert("verbose".to_string(), "debug".to_string()); // valid
                m.insert("custom".to_string(), "nonexistent_level".to_string()); // invalid
                m
            }),
            colors: None,
        };
        config.apply_file_config(file_config);
        let aliases = config.level_aliases.unwrap();
        assert_eq!(aliases.get("verbose"), Some(&Level::Debug));
        assert!(
            !aliases.contains_key("custom"),
            "invalid level alias should be silently skipped"
        );
    }

    #[test]
    fn test_apply_file_config_all_invalid_aliases_produces_none() {
        // If all level aliases are invalid, level_aliases should remain None
        let mut config = Config::default();
        let file_config = FileConfig {
            color: None,
            level: None,
            timestamp_format: None,
            max_field_length: None,
            line_gap: None,
            key_min_width: None,
            keys: None,
            levels: Some({
                let mut m = HashMap::new();
                m.insert("foo".to_string(), "not_a_level".to_string());
                m
            }),
            colors: None,
        };
        config.apply_file_config(file_config);
        assert!(
            config.level_aliases.is_none(),
            "all-invalid aliases should leave level_aliases as None"
        );
    }

    #[test]
    fn test_apply_file_config_valid_colors() {
        let mut config = Config::default();
        let file_config = FileConfig {
            color: None,
            level: None,
            timestamp_format: None,
            max_field_length: None,
            line_gap: None,
            key_min_width: None,
            keys: None,
            levels: None,
            colors: Some({
                let mut m = HashMap::new();
                m.insert("info".to_string(), "cyan".to_string());
                m.insert("error".to_string(), "bright_red".to_string());
                m
            }),
        };
        config.apply_file_config(file_config);
        let colors = config.level_colors.unwrap();
        assert_eq!(colors.get(&Level::Info), Some(&"cyan".to_string()));
        assert_eq!(colors.get(&Level::Error), Some(&"bright_red".to_string()));
    }

    #[test]
    fn test_apply_file_config_invalid_colors_skipped() {
        let mut config = Config::default();
        let file_config = FileConfig {
            color: None,
            level: None,
            timestamp_format: None,
            max_field_length: None,
            line_gap: None,
            key_min_width: None,
            keys: None,
            levels: None,
            colors: Some({
                let mut m = HashMap::new();
                m.insert("info".to_string(), "rainbow".to_string()); // invalid color
                m.insert("error".to_string(), "red".to_string()); // valid
                m
            }),
        };
        config.apply_file_config(file_config);
        let colors = config.level_colors.unwrap();
        assert!(
            !colors.contains_key(&Level::Info),
            "invalid color 'rainbow' should be silently skipped"
        );
        assert_eq!(colors.get(&Level::Error), Some(&"red".to_string()));
    }

    #[test]
    fn test_apply_file_config_all_invalid_colors_produces_none() {
        let mut config = Config::default();
        let file_config = FileConfig {
            color: None,
            level: None,
            timestamp_format: None,
            max_field_length: None,
            line_gap: None,
            key_min_width: None,
            keys: None,
            levels: None,
            colors: Some({
                let mut m = HashMap::new();
                m.insert("info".to_string(), "rainbow".to_string());
                m.insert("error".to_string(), "neon".to_string());
                m
            }),
        };
        config.apply_file_config(file_config);
        assert!(
            config.level_colors.is_none(),
            "all-invalid colors should leave level_colors as None"
        );
    }

    #[test]
    fn test_apply_file_config_invalid_level_in_colors_skipped() {
        // A valid color but for an unrecognized level name
        let mut config = Config::default();
        let file_config = FileConfig {
            color: None,
            level: None,
            timestamp_format: None,
            max_field_length: None,
            line_gap: None,
            key_min_width: None,
            keys: None,
            levels: None,
            colors: Some({
                let mut m = HashMap::new();
                m.insert("verbose".to_string(), "red".to_string()); // invalid level
                m.insert("warn".to_string(), "yellow".to_string()); // valid
                m
            }),
        };
        config.apply_file_config(file_config);
        let colors = config.level_colors.unwrap();
        assert_eq!(colors.len(), 1);
        assert_eq!(colors.get(&Level::Warn), Some(&"yellow".to_string()));
    }

    #[test]
    fn test_file_config_empty_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(&path, "").unwrap();
        let file_config = FileConfig::load(&path).unwrap();
        assert!(file_config.color.is_none());
        assert!(file_config.level.is_none());
        assert!(file_config.timestamp_format.is_none());
        assert!(file_config.max_field_length.is_none());
        assert!(file_config.line_gap.is_none());
        assert!(file_config.key_min_width.is_none());
        assert!(file_config.keys.is_none());
        assert!(file_config.levels.is_none());
        assert!(file_config.colors.is_none());
    }

    #[test]
    fn test_apply_file_config_unrecognized_color_defaults_to_auto() {
        let mut config = Config::default();
        let file_config = FileConfig {
            color: Some("invalid_value".to_string()),
            level: None,
            timestamp_format: None,
            max_field_length: None,
            line_gap: None,
            key_min_width: None,
            keys: None,
            levels: None,
            colors: None,
        };
        config.apply_file_config(file_config);
        assert_eq!(
            config.color_mode,
            ColorMode::Auto,
            "unrecognized color value should default to Auto"
        );
    }
}
