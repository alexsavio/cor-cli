//! Error types for the `cor` application.
//!
//! Uses [`thiserror`] for ergonomic error derivation.

use thiserror::Error;

/// Errors that can occur in `cor`.
///
/// Maps to exit codes: [`Config`](Self::Config) → exit 1,
/// [`Io`](Self::Io) → exit 2.
#[derive(Debug, Error)]
pub enum CorError {
    /// Configuration error (invalid flag combination, unreadable config file).
    #[error("configuration error: {0}")]
    Config(String),

    /// I/O error during read or write.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON parse error (informational, not user-facing in normal operation).
    #[error("parse error: {0}")]
    #[allow(dead_code)] // Available for future use in verbose/debug mode
    Parse(String),

    /// TOML deserialization error.
    #[error("config file error: {0}")]
    Toml(#[from] toml::de::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_error_display() {
        let err = CorError::Config("bad flag".into());
        assert_eq!(err.to_string(), "configuration error: bad flag");
    }

    #[test]
    fn test_io_error_display() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file missing");
        let err = CorError::Io(io_err);
        assert_eq!(err.to_string(), "I/O error: file missing");
    }

    #[test]
    fn test_parse_error_display() {
        let err = CorError::Parse("unexpected token".into());
        assert_eq!(err.to_string(), "parse error: unexpected token");
    }

    #[test]
    fn test_io_error_from_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied");
        let err: CorError = io_err.into();
        assert!(matches!(err, CorError::Io(_)));
    }
}
