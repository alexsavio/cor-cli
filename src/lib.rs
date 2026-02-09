//! `cor` — Colorize JSON-structured log lines from stdin.
//!
//! This library provides the core parsing and formatting functionality for
//! the `cor` CLI tool. It can parse JSON log lines from various frameworks
//! (logrus, zap, slog, pino, bunyan, structlog), auto-detect timestamp,
//! level, and message fields, and output colorized human-readable text.
//!
//! # Example
//!
//! ```
//! use cor::{Config, format_line};
//!
//! let config = Config::default();
//! let mut out = String::new();
//!
//! format_line(r#"{"level":"info","msg":"hello","port":8080}"#, &config, false, &mut out);
//! assert!(out.contains("INFO"));
//! assert!(out.contains("hello"));
//! ```

pub mod cli;
pub mod config;
pub mod error;
pub mod fields;
pub mod formatter;
pub mod level;
pub mod parser;
pub mod timestamp;

// Re-export primary API types for convenience.
pub use config::Config;
pub use error::CorError;
pub use formatter::{format_line, format_line_parsed};
pub use level::Level;
pub use parser::{LineKind, LogRecord, parse_line, sanitize_json_newlines, un_double_escape_json};
pub use timestamp::Timestamp;
