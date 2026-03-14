//! Integration tests for CLI flags: --line-gap, --verbose, --config errors, --level errors,
//! --help, --version, and combined flag scenarios.

use predicates::prelude::*;
use std::io::Write;

use super::cor;

// ── --line-gap ──────────────────────────────────────────────────────

#[test]
fn line_gap_zero_compact_output() {
    let input = r#"{"level":"info","msg":"first"}
{"level":"info","msg":"second"}
{"level":"info","msg":"third"}"#;
    let output = cor()
        .arg("--color=never")
        .arg("--line-gap=0")
        .write_stdin(input)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    // With line_gap=0, there should be no blank lines between entries
    assert!(
        !stdout.contains("\n\n"),
        "line-gap=0 should produce no blank lines between entries.\nGot: {stdout}"
    );
    assert!(stdout.contains("first"));
    assert!(stdout.contains("second"));
    assert!(stdout.contains("third"));
}

#[test]
fn line_gap_two_double_spaced() {
    let input = r#"{"level":"info","msg":"first"}
{"level":"info","msg":"second"}"#;
    let output = cor()
        .arg("--color=never")
        .arg("--line-gap=2")
        .write_stdin(input)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    // With line_gap=2, there should be 2 blank lines (3 newlines) between entries
    assert!(
        stdout.contains("\n\n\n"),
        "line-gap=2 should produce double-spaced output.\nGot: {stdout}"
    );
    assert!(stdout.contains("first"));
    assert!(stdout.contains("second"));
}

#[test]
fn line_gap_default_single_blank_line() {
    let input = r#"{"level":"info","msg":"first"}
{"level":"info","msg":"second"}"#;
    let output = cor()
        .arg("--color=never")
        .write_stdin(input)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Default line_gap=1: one blank line between entries (two consecutive newlines)
    assert!(
        stdout.contains("\n\n"),
        "default line-gap should produce one blank line between entries.\nGot: {stdout}"
    );
    // But not three consecutive newlines (that would be line_gap=2)
    assert!(
        !stdout.contains("\n\n\n"),
        "default line-gap should not produce double spacing.\nGot: {stdout}"
    );
}

// ── --verbose ───────────────────────────────────────────────────────

#[test]
fn verbose_shows_parse_error_in_binary() {
    let input = r#"{"level":"info", "msg":}"#; // Invalid JSON
    let output = cor()
        .arg("--color=never")
        .arg("--verbose")
        .write_stdin(input)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("parse error:"),
        "verbose flag should show parse error in binary output.\nGot: {stdout}"
    );
}

#[test]
fn verbose_no_error_for_valid_json() {
    let input = r#"{"level":"info","msg":"hello"}"#;
    let output = cor()
        .arg("--color=never")
        .arg("--verbose")
        .write_stdin(input)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("parse error"),
        "verbose should not show errors for valid JSON.\nGot: {stdout}"
    );
}

// ── --config with bad content ────────────────────────────────────────

#[test]
fn config_invalid_toml_exits_one() {
    let mut config_file = tempfile::NamedTempFile::new().unwrap();
    config_file.write_all(b"this is not valid [[ toml").unwrap();

    let input = r#"{"level":"info","msg":"hello"}"#;
    cor()
        .arg(format!("--config={}", config_file.path().display()))
        .write_stdin(input)
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("config file error"));
}

#[test]
fn config_nonexistent_path_exits_one() {
    let input = r#"{"level":"info","msg":"hello"}"#;
    cor()
        .arg("--config=/tmp/cor-test-nonexistent-path/config.toml")
        .write_stdin(input)
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("config file not found"));
}

// ── --level with invalid value ──────────────────────────────────────

#[test]
fn level_invalid_value_exits_two() {
    let input = r#"{"level":"info","msg":"hello"}"#;
    cor()
        .arg("--level=garbage")
        .write_stdin(input)
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("invalid level"));
}

// ── --help and --version ────────────────────────────────────────────

#[test]
fn help_flag_exits_zero() {
    cor()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Colorize JSON-structured log lines",
        ));
}

#[test]
fn version_flag_exits_zero() {
    cor()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("cor "));
}

// ── Combined flags ──────────────────────────────────────────────────

#[test]
fn json_mode_with_level_filter_and_embedded_json() {
    let input = r#"prefix {"level":"info","msg":"info msg"}
prefix {"level":"warn","msg":"warn msg"}
prefix {"level":"error","msg":"error msg"}"#;
    let output = cor()
        .arg("--color=never")
        .arg("--json")
        .arg("--level=warn")
        .write_stdin(input)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("info msg"),
        "info should be filtered with --json --level=warn"
    );
    assert!(
        stdout.contains("warn msg"),
        "warn should pass with --json --level=warn"
    );
    assert!(
        stdout.contains("error msg"),
        "error should pass with --json --level=warn"
    );
    // Output should be JSON
    assert!(
        stdout.contains(r#""level":"warn""#),
        "output should be raw JSON"
    );
}

// ── Config file with logger/caller/error keys ───────────────────────

#[test]
fn config_file_logger_caller_error_keys() {
    let config_content = r#"
[keys]
logger = "service"
caller = "loc"
error = "err_msg"
"#;
    let mut config_file = tempfile::NamedTempFile::new().unwrap();
    config_file.write_all(config_content.as_bytes()).unwrap();

    let input = r#"{"level":"error","msg":"failed","service":"payments","loc":"handler.go:42","err_msg":"connection timeout"}"#;
    let output = cor()
        .arg("--color=never")
        .arg(format!("--config={}", config_file.path().display()))
        .write_stdin(input)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Logger should be extracted and shown (dimmed in color mode, plain in --color=never)
    assert!(
        stdout.contains("payments"),
        "logger should be extracted from custom key.\nGot: {stdout}"
    );
    // Caller should appear in parentheses
    assert!(
        stdout.contains("(handler.go:42)"),
        "caller should be extracted from custom key.\nGot: {stdout}"
    );
    // Error should appear with error label
    assert!(
        stdout.contains("connection timeout"),
        "error should be extracted from custom key.\nGot: {stdout}"
    );
    // These custom fields should NOT appear as extra key-value pairs
    assert!(
        !stdout.contains("service:") && !stdout.contains("loc:") && !stdout.contains("err_msg:"),
        "custom key fields should not appear as extra.\nGot: {stdout}"
    );
}

// ── Unicode in field values ─────────────────────────────────────────

#[test]
fn unicode_in_field_values() {
    let input = r#"{"level":"info","msg":"你好世界","user":"José 🎉","tag":"日本語"}"#;
    let output = cor()
        .arg("--color=never")
        .write_stdin(input)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("你好世界"),
        "CJK message should pass through"
    );
    assert!(
        stdout.contains("José 🎉"),
        "accented chars and emoji should pass through"
    );
    assert!(
        stdout.contains("日本語"),
        "Japanese chars in extra fields should pass through"
    );
}
