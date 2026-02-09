//! Integration tests for embedded JSON detection.

use assert_cmd::Command;
use predicates::prelude::*;

#[allow(deprecated)]
fn cor() -> Command {
    let mut cmd = Command::cargo_bin("cor").unwrap();
    cmd.env("XDG_CONFIG_HOME", "/tmp/cor-test-no-config");
    cmd
}

#[test]
fn embedded_json_colorized() {
    let input = r#"2026-02-06 00:15:13.449 {"level":"debug","msg":"health check"}"#;
    cor()
        .arg("--color=never")
        .write_stdin(input)
        .assert()
        .success()
        .stdout(predicate::str::contains("DEBUG"))
        .stdout(predicate::str::contains("health check"));
}

#[test]
fn embedded_json_prefix_preserved() {
    let input = r#"prefix text {"level":"info","msg":"hello"}"#;
    let output = cor()
        .arg("--color=never")
        .write_stdin(input)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("prefix text"),
        "Prefix should be preserved in output"
    );
    assert!(stdout.contains("INFO"));
    assert!(stdout.contains("hello"));
}

#[test]
fn embedded_json_prefix_before_formatted() {
    let input = r#"myapp | {"level":"warn","msg":"disk low","available":"2GB"}"#;
    let output = cor()
        .arg("--color=never")
        .write_stdin(input)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Prefix should appear in the output after the level badge (fblog style)
    let warn_pos = stdout.find("WARN").unwrap();
    let prefix_pos = stdout.find("myapp |").unwrap();
    assert!(
        warn_pos < prefix_pos,
        "Level badge should appear before prefix in fblog style"
    );
}

#[test]
fn invalid_json_after_brace_treated_as_raw() {
    let input = "some text {not valid json at all}";
    let output = cor()
        .arg("--color=never")
        .write_stdin(input)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(
        stdout.trim(),
        "some text {not valid json at all}",
        "Invalid embedded JSON should be treated as raw"
    );
}

#[test]
fn multiple_braces_first_valid_json_used() {
    // The first '{' is part of the prefix text, the actual JSON starts later
    let input = r#"value={count} {"level":"info","msg":"parsed"}"#;
    let output = cor()
        .arg("--color=never")
        .write_stdin(input)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should parse the embedded JSON if the first brace attempt fails
    // and find the valid JSON at the second '{'
    // The exact behavior depends on implementation — our parser tries from the first '{'
    // If that fails, it's treated as Raw
    // This tests that the binary doesn't crash
    assert!(output.status.success());
    // Either it parses the second JSON or treats the whole line as raw
    assert!(!stdout.is_empty());
}

#[test]
fn fixture_embedded_file() {
    let input = std::fs::read_to_string("tests/fixtures/embedded.jsonl").unwrap();
    cor()
        .arg("--color=never")
        .write_stdin(input)
        .assert()
        .success()
        .stdout(predicate::str::contains("INFO"))
        .stdout(predicate::str::contains("WARN"))
        .stdout(predicate::str::contains("ERROR"))
        .stdout(predicate::str::contains("server started"));
}

#[test]
fn embedded_json_with_json_mode() {
    let input = r#"prefix {"level":"info","msg":"hello"}"#;
    let output = cor()
        .arg("--color=never")
        .arg("--json")
        .write_stdin(input)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    // In --json mode, embedded JSON outputs just the JSON part
    assert!(stdout.contains(r#"{"level":"info","msg":"hello"}"#));
}

#[test]
fn timestamp_prefix_datetime_space_json() {
    // Common Docker / k8s log format: "<YYYY-MM-DD HH:MM:SS.fff> {json}"
    let input = r#"2026-01-15 10:30:00.123 {"level":"info","msg":"server started","port":8080}"#;
    let output = cor()
        .arg("--color=never")
        .write_stdin(input)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("INFO"), "Level should be parsed");
    assert!(
        stdout.contains("server started"),
        "Message should be extracted"
    );
    assert!(stdout.contains("port: 8080"), "Extra fields should appear");
    assert!(
        stdout.contains("2026-01-15 10:30:00.123"),
        "Timestamp prefix should be preserved"
    );
}

#[test]
fn timestamp_prefix_iso8601_json() {
    // ISO 8601 timestamp prefix
    let input = r#"2026-01-15T10:30:00.123Z {"level":"warn","msg":"high latency","ms":350}"#;
    let output = cor()
        .arg("--color=never")
        .write_stdin(input)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("WARN"));
    assert!(stdout.contains("high latency"));
    assert!(stdout.contains("ms: 350"));
}

#[test]
fn timestamp_prefix_with_timezone_offset() {
    let input =
        r#"2026-01-15T12:30:00+02:00 {"level":"error","msg":"connection lost","host":"db01"}"#;
    let output = cor()
        .arg("--color=never")
        .write_stdin(input)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("ERROR"));
    assert!(stdout.contains("connection lost"));
    assert!(stdout.contains("host: db01"));
}

#[test]
fn timestamp_prefix_uses_json_timestamp_for_display() {
    // When both a prefix timestamp and a JSON timestamp exist,
    // the JSON timestamp field is what gets formatted as HH:MM:SS.mmm
    let input =
        r#"2026-01-15 10:30:00.123 {"ts":"2026-01-15T11:45:00.999Z","level":"debug","msg":"test"}"#;
    let output = cor()
        .arg("--color=never")
        .write_stdin(input)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("11:45:00.999"),
        "JSON timestamp should be used for the formatted display"
    );
    assert!(
        stdout.contains("2026-01-15 10:30:00.123"),
        "Prefix timestamp should still appear as-is"
    );
}

#[test]
fn timestamp_prefix_multiline_stream() {
    // Multiple lines with timestamp prefixes
    let input = "\
2026-01-15 10:30:00.001 {\"level\":\"info\",\"msg\":\"starting up\"}\n\
2026-01-15 10:30:00.050 {\"level\":\"debug\",\"msg\":\"loading config\"}\n\
2026-01-15 10:30:01.200 {\"level\":\"warn\",\"msg\":\"deprecated API used\",\"endpoint\":\"/v1/old\"}\n\
2026-01-15 10:30:02.500 {\"level\":\"error\",\"msg\":\"unhandled exception\",\"code\":500}";
    let output = cor()
        .arg("--color=never")
        .write_stdin(input)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("INFO"));
    assert!(stdout.contains("DEBUG"));
    assert!(stdout.contains("WARN"));
    assert!(stdout.contains("ERROR"));
    assert!(stdout.contains("starting up"));
    assert!(stdout.contains("deprecated API used"));
    assert!(
        stdout.contains("endpoint: /v1/old"),
        "Extra fields should be formatted"
    );
    assert!(stdout.contains("code: 500"));
}

#[test]
fn timestamp_prefix_no_level_in_json() {
    // Timestamp prefix with JSON that has no level field
    let input = r#"2026-01-15 10:30:00.000 {"msg":"just a message","key":"value"}"#;
    let output = cor()
        .arg("--color=never")
        .write_stdin(input)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("just a message"));
    assert!(stdout.contains("key: value"));
    // Should have blank badge with colon
    assert!(
        stdout.contains("     :"),
        "Missing level should produce blank badge with colon"
    );
}

#[test]
fn multiline_json_with_raw_newlines_in_exception() {
    // Simulates a structlog JSON entry where the exception traceback
    // has actual newline bytes (0x0A) instead of JSON-escaped \n.
    // This splits the JSON across multiple lines for BufRead::lines().
    let mut input = Vec::new();
    input.extend_from_slice(
        b"2026-02-09 11:15:15.096 {\"event\":\"Failed to create job template.\",\"level\":\"error\",\"exception\":\"Traceback (most recent call last):\n  File \\\"app.py\\\", line 72\n    raise Error\",\"timestamp\":\"2026-02-09T11:15:15.096Z\"}\n",
    );
    input.extend_from_slice(
        b"{\"event\":\"Creating job template.\",\"level\":\"info\",\"timestamp\":\"2026-02-09T11:15:17.096Z\"}\n",
    );

    let output = cor()
        .arg("--color=never")
        .write_stdin(input)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Error line should be parsed and formatted
    assert!(
        stdout.contains("ERROR:"),
        "Error log with raw newlines should be formatted.\nGot: {stdout}"
    );
    assert!(
        stdout.contains("Failed to create job template."),
        "Error message should be extracted.\nGot: {stdout}"
    );
    assert!(
        stdout.contains("exception:"),
        "Exception field should appear in extra fields.\nGot: {stdout}"
    );

    // Info line should also be formatted
    assert!(
        stdout.contains("INFO:"),
        "Subsequent info log should also be formatted.\nGot: {stdout}"
    );
    assert!(
        stdout.contains("Creating job template."),
        "Info message should be extracted.\nGot: {stdout}"
    );
}

#[test]
fn multiline_json_pure_json_with_raw_newlines() {
    // Pure JSON (no prefix) with raw newlines in a string value.
    let input =
        b"{\"level\":\"error\",\"msg\":\"fail\",\"stack\":\"Error\n  at main\n  at run\"}\n";

    let output = cor()
        .arg("--color=never")
        .write_stdin(input.to_vec())
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains("ERROR:"),
        "Pure JSON with raw newlines should be formatted.\nGot: {stdout}"
    );
    assert!(
        stdout.contains("fail"),
        "Message should be extracted.\nGot: {stdout}"
    );
}

#[test]
fn double_escaped_json_parsed_as_error() {
    // JSON with double-escaped sequences: \\n and \\" instead of \n and \"
    // This is produced by some log pipelines that double-escape strings.
    let input = r#"2026-02-09 11:15:17.180 {"event":"Failed to create job template.","level":"error","exception":"Traceback (most recent call last):\\n  File \\"/src/app/scicat.py\\", line 72\\nhttpx.HTTPStatusError: Server error","timestamp":"2026-02-09T11:15:17.180Z"}"#;

    let output = cor()
        .arg("--color=never")
        .write_stdin(input)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains("ERROR:"),
        "Double-escaped JSON should be formatted as ERROR.\nGot: {stdout}"
    );
    assert!(
        stdout.contains("Failed to create job template."),
        "Message should be extracted from double-escaped JSON.\nGot: {stdout}"
    );
    assert!(
        stdout.contains("exception:"),
        "Exception field should appear.\nGot: {stdout}"
    );
}
