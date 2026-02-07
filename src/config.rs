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
            timestamp_format: "%H:%M:%S%.3f".to_string(),
            level_aliases: None,
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

        // CLI overrides
        config.color_mode = cli.color;

        if let Some(ref level_str) = cli.level {
            config.min_level = Level::from_str_loose(level_str);
        }

        if cli.message_key.is_some() {
            config.message_key.clone_from(&cli.message_key);
        }
        if cli.level_key.is_some() {
            config.level_key.clone_from(&cli.level_key);
        }
        if cli.timestamp_key.is_some() {
            config.timestamp_key.clone_from(&cli.timestamp_key);
        }
        if cli.include_fields.is_some() {
            config.include_fields.clone_from(&cli.include_fields);
        }
        if cli.exclude_fields.is_some() {
            config.exclude_fields.clone_from(&cli.exclude_fields);
        }

        config.json_output = cli.json;
        config.max_field_length = cli.max_field_length;

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
    }
}

/// Config file structure (TOML deserialization).
#[derive(Debug, Deserialize)]
struct FileConfig {
    color: Option<String>,
    level: Option<String>,
    timestamp_format: Option<String>,
    max_field_length: Option<usize>,
    keys: Option<KeysConfig>,
    levels: Option<HashMap<String, String>>,
    #[allow(dead_code)] // Parsed but not yet used — will support custom level colors
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
        assert!(!config.json_output);
    }

    #[test]
    fn test_file_config_parse() {
        let toml_str = r#"
            color = "always"
            level = "warn"
            timestamp_format = "%H:%M:%S"
            max_field_length = 80

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
        assert!(config.level_aliases.is_some());
    }
}
