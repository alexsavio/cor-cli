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
