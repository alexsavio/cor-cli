//! Integration tests for mixed JSON + non-JSON input.

use assert_cmd::Command;

#[allow(deprecated)]
fn cor() -> Command {
    let mut cmd = Command::cargo_bin("cor").unwrap();
    cmd.env("XDG_CONFIG_HOME", "/tmp/cor-test-no-config");
    cmd
}

#[test]
fn json_and_plain_text_mixed() {
    let input = r#"Starting application...
{"level":"info","msg":"server started","port":8080}
Plain text log line
{"level":"error","msg":"connection failed"}
Shutting down."#;

    let output = cor()
        .arg("--color=never")
        .write_stdin(input)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Plain text lines pass through unchanged
    assert!(stdout.contains("Starting application..."));
    assert!(stdout.contains("Plain text log line"));
    assert!(stdout.contains("Shutting down."));

    // JSON lines are formatted
    assert!(stdout.contains("INFO"));
    assert!(stdout.contains("server started"));
    assert!(stdout.contains("ERROR"));
    assert!(stdout.contains("connection failed"));
}

#[test]
fn malformed_json_passthrough() {
    let input = r#"{"level":"info", "msg":}
{"level":"info","msg":"valid line"}"#;

    let output = cor()
        .arg("--color=never")
        .write_stdin(input)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Malformed JSON passes through unchanged
    assert!(stdout.contains(r#"{"level":"info", "msg":}"#));
    // Valid JSON is formatted
    assert!(stdout.contains("INFO"));
    assert!(stdout.contains("valid line"));
}

#[test]
fn json_array_passthrough_as_raw() {
    let input = r#"[1, 2, 3]
{"level":"info","msg":"after array"}"#;

    let output = cor()
        .arg("--color=never")
        .write_stdin(input)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);

    // JSON arrays pass through as raw text
    assert!(stdout.contains("[1, 2, 3]"));
    // Valid JSON object is formatted
    assert!(stdout.contains("INFO"));
    assert!(stdout.contains("after array"));
}

#[test]
fn no_recognized_fields_renders_key_value() {
    let input = r#"{"custom_a":"value_a","custom_b":42}"#;

    let output = cor()
        .arg("--color=never")
        .write_stdin(input)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);

    // All fields rendered as key: value
    assert!(stdout.contains("custom_a: value_a"));
    assert!(stdout.contains("custom_b: 42"));
}

#[test]
fn empty_json_object_handled() {
    let input = "{}";
    cor()
        .arg("--color=never")
        .write_stdin(input)
        .assert()
        .success();
}
