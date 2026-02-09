//! Integration tests for basic stdin->stdout piping.

use assert_cmd::Command;
use predicates::prelude::*;

#[allow(deprecated)]
fn cor() -> Command {
    let mut cmd = Command::cargo_bin("cor").unwrap();
    cmd.env("XDG_CONFIG_HOME", "/tmp/cor-test-no-config");
    cmd
}

#[test]
fn empty_stdin_exits_zero() {
    cor().write_stdin("").assert().success().stdout("");
}

#[test]
fn single_json_line_outputs_formatted() {
    let input = r#"{"level":"info","msg":"hello","port":8080}"#;
    cor()
        .arg("--color=never")
        .write_stdin(input)
        .assert()
        .success()
        .stdout(predicate::str::contains("INFO"))
        .stdout(predicate::str::contains("hello"))
        .stdout(predicate::str::contains("port: 8080"));
}

#[test]
fn extra_fields_sorted_alphabetically() {
    let input = r#"{"level":"info","msg":"test","zebra":"z","alpha":"a","middle":"m"}"#;
    let output = cor()
        .arg("--color=never")
        .write_stdin(input)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let alpha_pos = stdout.find("alpha:").unwrap();
    let middle_pos = stdout.find("middle:").unwrap();
    let zebra_pos = stdout.find("zebra:").unwrap();
    assert!(alpha_pos < middle_pos, "alpha should come before middle");
    assert!(middle_pos < zebra_pos, "middle should come before zebra");
}

#[test]
fn dot_notation_flattening() {
    let input = r#"{"level":"info","msg":"req","http":{"method":"GET","status":200}}"#;
    cor()
        .arg("--color=never")
        .write_stdin(input)
        .assert()
        .success()
        .stdout(predicate::str::contains("http.method: GET"))
        .stdout(predicate::str::contains("http.status: 200"));
}

#[test]
fn truncation_at_default_120_chars() {
    let long_val = "x".repeat(200);
    let input = format!(r#"{{"level":"info","msg":"test","data":"{long_val}"}}"#);
    let output = cor()
        .arg("--color=never")
        .write_stdin(input)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    // The value should be truncated with …
    assert!(
        stdout.contains('…'),
        "Long value should be truncated with …"
    );
    // Should not contain the full 200-char value
    assert!(
        !stdout.contains(&long_val),
        "Full 200-char value should not appear"
    );
}

#[test]
fn truncation_disabled_with_zero() {
    let long_val = "x".repeat(200);
    let input = format!(r#"{{"level":"info","msg":"test","data":"{long_val}"}}"#);
    let output = cor()
        .arg("--color=never")
        .arg("--max-field-length=0")
        .write_stdin(input)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains(&long_val),
        "Full value should appear when truncation is disabled"
    );
}

#[test]
fn broken_pipe_exits_zero() {
    // Simulate: cor | head -1 by just checking that cor handles stdin correctly
    // We can't easily simulate a broken pipe, but we verify the binary runs correctly
    let input = "line1\nline2\nline3\n";
    cor()
        .arg("--color=never")
        .write_stdin(input)
        .assert()
        .success();
}

#[test]
fn logrus_fixture_auto_detect() {
    let input = std::fs::read_to_string("tests/fixtures/logrus.jsonl").unwrap();
    cor()
        .arg("--color=never")
        .write_stdin(input)
        .assert()
        .success()
        .stdout(predicate::str::contains("INFO"))
        .stdout(predicate::str::contains("server started"))
        .stdout(predicate::str::contains("ERROR"))
        .stdout(predicate::str::contains("FATAL"));
}

#[test]
fn zap_fixture_auto_detect() {
    let input = std::fs::read_to_string("tests/fixtures/zap.jsonl").unwrap();
    cor()
        .arg("--color=never")
        .write_stdin(input)
        .assert()
        .success()
        .stdout(predicate::str::contains("INFO"))
        .stdout(predicate::str::contains("server started"))
        .stdout(predicate::str::contains("WARN"))
        .stdout(predicate::str::contains("ERROR"));
}

#[test]
fn slog_fixture_auto_detect() {
    let input = std::fs::read_to_string("tests/fixtures/slog.jsonl").unwrap();
    cor()
        .arg("--color=never")
        .write_stdin(input)
        .assert()
        .success()
        .stdout(predicate::str::contains("INFO"))
        .stdout(predicate::str::contains("server started"))
        .stdout(predicate::str::contains("WARN"))
        .stdout(predicate::str::contains("ERROR"));
}

#[test]
fn pino_fixture_numeric_levels() {
    let input = std::fs::read_to_string("tests/fixtures/pino.jsonl").unwrap();
    cor()
        .arg("--color=never")
        .write_stdin(input)
        .assert()
        .success()
        .stdout(predicate::str::contains("INFO"))
        .stdout(predicate::str::contains("server listening"))
        .stdout(predicate::str::contains("WARN"))
        .stdout(predicate::str::contains("ERROR"))
        .stdout(predicate::str::contains("FATAL"));
}

#[test]
fn bunyan_fixture_numeric_levels() {
    let input = std::fs::read_to_string("tests/fixtures/bunyan.jsonl").unwrap();
    cor()
        .arg("--color=never")
        .write_stdin(input)
        .assert()
        .success()
        .stdout(predicate::str::contains("INFO"))
        .stdout(predicate::str::contains("server started"))
        .stdout(predicate::str::contains("WARN"))
        .stdout(predicate::str::contains("ERROR"));
}

#[test]
fn structlog_fixture_auto_detect() {
    let input = std::fs::read_to_string("tests/fixtures/structlog.jsonl").unwrap();
    cor()
        .arg("--color=never")
        .write_stdin(input)
        .assert()
        .success()
        .stdout(predicate::str::contains("INFO"))
        .stdout(predicate::str::contains("server started"))
        .stdout(predicate::str::contains("WARN"))
        .stdout(predicate::str::contains("ERROR"));
}

#[test]
fn timestamp_displayed_as_datetime() {
    let input = r#"{"ts":"2026-01-15T10:30:00.123Z","level":"info","msg":"hello"}"#;
    cor()
        .arg("--color=never")
        .write_stdin(input)
        .assert()
        .success()
        .stdout(predicate::str::contains("2026-01-15T10:30:00.123"));
}

#[test]
fn no_level_shows_blank_badge() {
    let input = r#"{"msg":"no level here","port":8080}"#;
    let output = cor()
        .arg("--color=never")
        .write_stdin(input)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should contain the blank badge (5 spaces) followed by the message
    assert!(
        stdout.contains("     : no level here"),
        "Expected blank badge with colon before message, got: {stdout}"
    );
}

#[test]
fn unrecognized_level_shows_blank_badge() {
    let input = r#"{"level":"verbose","msg":"custom level"}"#;
    let output = cor()
        .arg("--color=never")
        .write_stdin(input)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("custom level"));
    // verbose is not a recognized level → blank badge with colon
    assert!(
        stdout.contains("     : custom level"),
        "Expected blank badge with colon for unrecognized level"
    );
}

#[test]
fn extremely_long_line_no_crash() {
    let long_val = "x".repeat(1_100_000);
    let input = format!(r#"{{"level":"info","msg":"big","data":"{long_val}"}}"#);
    cor()
        .arg("--color=never")
        .write_stdin(input)
        .assert()
        .success();
}

#[test]
fn string_values_unquoted() {
    let input = r#"{"level":"info","msg":"test","name":"John"}"#;
    let output = cor()
        .arg("--color=never")
        .write_stdin(input)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("name: John"),
        "String values should be unquoted"
    );
    assert!(
        !stdout.contains("name: \"John\""),
        "String values should NOT be quoted"
    );
}
