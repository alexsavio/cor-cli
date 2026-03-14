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

// ── --timestamp-format ────────────────────────────────────────────

#[test]
fn timestamp_format_time_only() {
    let input = r#"{"level":"info","msg":"hello","time":"2026-01-15T10:30:00.123Z"}"#;
    let output = cor()
        .arg("--color=never")
        .arg("--timestamp-format=%H:%M:%S")
        .write_stdin(input)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("10:30:00"),
        "time-only format should show hours/minutes/seconds.\nGot: {stdout}"
    );
    assert!(
        !stdout.contains("2026-01-15"),
        "time-only format should not contain the date.\nGot: {stdout}"
    );
}

// ── --key-min-width ───────────────────────────────────────────────

#[test]
fn key_min_width_renders_correctly() {
    let input = r#"{"level":"info","msg":"hi","port":8080,"host":"localhost"}"#;
    let output = cor()
        .arg("--color=never")
        .arg("--key-min-width=10")
        .write_stdin(input)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("port: 8080"),
        "key-min-width should render key-value pairs.\nGot: {stdout}"
    );
    assert!(
        stdout.contains("host: localhost"),
        "key-min-width should render key-value pairs.\nGot: {stdout}"
    );
}

// ── --no-extra ────────────────────────────────────────────────────

#[test]
fn no_extra_hides_extra_fields() {
    let input = r#"{"level":"info","msg":"hello","port":8080,"host":"localhost"}"#;
    let output = cor()
        .arg("--color=never")
        .arg("--no-extra")
        .write_stdin(input)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("hello"),
        "message should still appear with --no-extra.\nGot: {stdout}"
    );
    assert!(
        stdout.contains("INFO"),
        "level should still appear with --no-extra.\nGot: {stdout}"
    );
    assert!(
        !stdout.contains("port"),
        "extra field 'port' should be hidden with --no-extra.\nGot: {stdout}"
    );
    assert!(
        !stdout.contains("host"),
        "extra field 'host' should be hidden with --no-extra.\nGot: {stdout}"
    );
}

#[test]
fn no_extra_still_shows_error_field() {
    let input = r#"{"level":"error","msg":"fail","port":8080,"error":"connection refused"}"#;
    let output = cor()
        .arg("--color=never")
        .arg("--no-extra")
        .write_stdin(input)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("connection refused"),
        "error field should still render with --no-extra.\nGot: {stdout}"
    );
    assert!(
        !stdout.contains("port"),
        "extra field 'port' should be hidden with --no-extra.\nGot: {stdout}"
    );
}

// ── --single-line ─────────────────────────────────────────────────

#[test]
fn single_line_inline_format() {
    let input = r#"{"level":"info","msg":"hello","port":8080,"host":"localhost"}"#;
    let output = cor()
        .arg("--color=never")
        .arg("--single-line")
        .write_stdin(input)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    // In single-line mode, extra fields use key=val format on the same line
    assert!(
        stdout.contains("port=8080"),
        "single-line should use key=val format.\nGot: {stdout}"
    );
    assert!(
        stdout.contains("host=localhost"),
        "single-line should use key=val format.\nGot: {stdout}"
    );
    // Message and extras should be on the same line
    let first_line = stdout.lines().next().unwrap_or("");
    assert!(
        first_line.contains("hello") && first_line.contains("port="),
        "message and extras should be on the same line.\nGot: {stdout}"
    );
}

#[test]
fn single_line_with_error_shows_first_line_only() {
    let input =
        r#"{"level":"error","msg":"fail","error":"Traceback:\n  File app.py\n    raise Error"}"#;
    let output = cor()
        .arg("--color=never")
        .arg("--single-line")
        .write_stdin(input)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    // In single-line mode, error should appear inline with first line only
    let first_line = stdout.lines().next().unwrap_or("");
    assert!(
        first_line.contains("error="),
        "error should appear inline in single-line mode.\nGot first line: {first_line}"
    );
    assert!(
        first_line.contains("Traceback:"),
        "first line of error should appear inline.\nGot first line: {first_line}"
    );
}

// ── --timezone ────────────────────────────────────────────────────

#[test]
fn timezone_utc_explicit() {
    let input = r#"{"level":"info","msg":"hello","time":"2026-01-15T10:30:00Z"}"#;
    let output = cor()
        .arg("--color=never")
        .arg("--timezone=UTC")
        .write_stdin(input)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("10:30:00"),
        "UTC timezone should preserve the original time.\nGot: {stdout}"
    );
}

#[test]
fn timezone_local_does_not_error() {
    let input = r#"{"level":"info","msg":"hello","time":"2026-01-15T10:30:00Z"}"#;
    cor()
        .arg("--color=never")
        .arg("--timezone=local")
        .write_stdin(input)
        .assert()
        .success();
}

#[test]
fn timezone_named_converts_timestamp() {
    let input = r#"{"level":"info","msg":"hello","time":"2026-01-15T10:30:00Z"}"#;
    let output = cor()
        .arg("--color=never")
        .arg("--timezone=Europe/London")
        .write_stdin(input)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    // January in London is UTC+0, so the time should remain 10:30:00
    assert!(
        stdout.contains("10:30:00"),
        "Europe/London in January should be UTC+0.\nGot: {stdout}"
    );
}

// ── --completions ─────────────────────────────────────────────────

#[test]
fn completions_bash_produces_output() {
    cor()
        .arg("--completions=bash")
        .assert()
        .success()
        .stdout(predicate::str::is_empty().not());
}

// ── File arguments ────────────────────────────────────────────────

#[test]
fn file_argument_reads_fixture() {
    let fixture = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/zap.jsonl");
    let output = cor().arg("--color=never").arg(fixture).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("server started"),
        "should parse lines from fixture file.\nGot: {stdout}"
    );
    assert!(
        stdout.contains("connection lost"),
        "should include error-level lines from fixture.\nGot: {stdout}"
    );
}

#[test]
fn file_argument_nonexistent_errors() {
    cor()
        .arg("--color=never")
        .arg("/tmp/cor-test-nonexistent-file.jsonl")
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::is_empty().not());
}

#[test]
fn file_argument_dash_reads_stdin() {
    let input = r#"{"level":"info","msg":"from stdin"}"#;
    let output = cor()
        .arg("--color=never")
        .arg("-")
        .write_stdin(input)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("from stdin"),
        "'-' should read from stdin.\nGot: {stdout}"
    );
}

// ── --grep ────────────────────────────────────────────────────────

#[test]
fn grep_matching_line_passes() {
    let input = r#"{"level":"info","msg":"hello world"}
{"level":"info","msg":"goodbye world"}"#;
    let output = cor()
        .arg("--color=never")
        .arg("--grep=hello")
        .write_stdin(input)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("hello world"),
        "matching line should pass through grep filter.\nGot: {stdout}"
    );
    assert!(
        !stdout.contains("goodbye"),
        "non-matching line should be filtered by grep.\nGot: {stdout}"
    );
}

#[test]
fn grep_non_matching_line_filtered() {
    let input = r#"{"level":"info","msg":"hello world"}"#;
    let output = cor()
        .arg("--color=never")
        .arg("--grep=nomatch")
        .write_stdin(input)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.trim().is_empty(),
        "non-matching line should produce no output.\nGot: {stdout}"
    );
}

#[test]
fn grep_filters_raw_lines_too() {
    let input = "this is a raw line\nanother raw line with keyword";
    let output = cor()
        .arg("--color=never")
        .arg("--grep=keyword")
        .write_stdin(input)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("keyword"),
        "raw line matching grep should pass.\nGot: {stdout}"
    );
    assert!(
        !stdout.contains("this is a raw line"),
        "raw line not matching grep should be filtered.\nGot: {stdout}"
    );
}

#[test]
fn grep_invalid_regex_exits_with_error() {
    let input = r#"{"level":"info","msg":"hello"}"#;
    cor()
        .arg("--color=never")
        .arg("--grep=[invalid")
        .write_stdin(input)
        .assert()
        .failure()
        .stderr(predicate::str::is_empty().not());
}

// ── --no-extra conflicts ──────────────────────────────────────────

#[test]
fn no_extra_conflicts_with_include_fields() {
    let input = r#"{"level":"info","msg":"hello"}"#;
    cor()
        .arg("--color=never")
        .arg("--no-extra")
        .arg("--include-fields=x")
        .write_stdin(input)
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("cannot be used with"));
}
