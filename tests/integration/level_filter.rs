//! Integration tests for level filtering (US2).

use assert_cmd::Command;

#[allow(deprecated)]
fn cor() -> Command {
    let mut cmd = Command::cargo_bin("cor").unwrap();
    cmd.env("XDG_CONFIG_HOME", "/tmp/cor-test-no-config");
    cmd
}

#[test]
fn level_warn_shows_warn_error_fatal() {
    let input = r#"{"level":"debug","msg":"debug msg"}
{"level":"info","msg":"info msg"}
{"level":"warn","msg":"warn msg"}
{"level":"error","msg":"error msg"}
{"level":"fatal","msg":"fatal msg"}"#;

    let output = cor()
        .arg("--color=never")
        .arg("--level=warn")
        .write_stdin(input)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(!stdout.contains("debug msg"), "debug should be filtered");
    assert!(!stdout.contains("info msg"), "info should be filtered");
    assert!(stdout.contains("warn msg"), "warn should pass");
    assert!(stdout.contains("error msg"), "error should pass");
    assert!(stdout.contains("fatal msg"), "fatal should pass");
}

#[test]
fn level_trace_shows_all() {
    let input = r#"{"level":"trace","msg":"trace msg"}
{"level":"debug","msg":"debug msg"}
{"level":"info","msg":"info msg"}
{"level":"warn","msg":"warn msg"}
{"level":"error","msg":"error msg"}
{"level":"fatal","msg":"fatal msg"}"#;

    let output = cor()
        .arg("--color=never")
        .arg("--level=trace")
        .write_stdin(input)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("trace msg"));
    assert!(stdout.contains("debug msg"));
    assert!(stdout.contains("info msg"));
    assert!(stdout.contains("warn msg"));
    assert!(stdout.contains("error msg"));
    assert!(stdout.contains("fatal msg"));
}

#[test]
fn no_level_flag_shows_all() {
    let input = r#"{"level":"debug","msg":"debug msg"}
{"level":"info","msg":"info msg"}
{"level":"error","msg":"error msg"}"#;

    let output = cor()
        .arg("--color=never")
        .write_stdin(input)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("debug msg"));
    assert!(stdout.contains("info msg"));
    assert!(stdout.contains("error msg"));
}

#[test]
fn numeric_levels_filtered_correctly() {
    // Pino/bunyan numeric levels: 20=debug, 30=info, 40=warn, 50=error
    let input = r#"{"level":20,"msg":"debug msg"}
{"level":30,"msg":"info msg"}
{"level":40,"msg":"warn msg"}
{"level":50,"msg":"error msg"}"#;

    let output = cor()
        .arg("--color=never")
        .arg("--level=warn")
        .write_stdin(input)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        !stdout.contains("debug msg"),
        "numeric debug should be filtered"
    );
    assert!(
        !stdout.contains("info msg"),
        "numeric info should be filtered"
    );
    assert!(stdout.contains("warn msg"), "numeric warn should pass");
    assert!(stdout.contains("error msg"), "numeric error should pass");
}

#[test]
fn non_json_lines_always_pass_through_during_filtering() {
    let input = r#"Plain text line
{"level":"debug","msg":"debug msg"}
Another plain line
{"level":"error","msg":"error msg"}"#;

    let output = cor()
        .arg("--color=never")
        .arg("--level=error")
        .write_stdin(input)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Non-JSON lines always pass through (FR-015)
    assert!(
        stdout.contains("Plain text line"),
        "Non-JSON should pass through during level filtering"
    );
    assert!(
        stdout.contains("Another plain line"),
        "Non-JSON should pass through during level filtering"
    );
    // Debug should be filtered
    assert!(!stdout.contains("debug msg"), "debug should be filtered");
    // Error should pass
    assert!(stdout.contains("error msg"), "error should pass");
}

#[test]
fn level_flag_case_insensitive() {
    let input = r#"{"level":"info","msg":"info msg"}
{"level":"error","msg":"error msg"}"#;

    let output = cor()
        .arg("--color=never")
        .arg("--level=ERROR")
        .write_stdin(input)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(!stdout.contains("info msg"));
    assert!(stdout.contains("error msg"));
}

#[test]
fn level_filter_with_json_output() {
    let input = r#"{"level":"debug","msg":"debug msg"}
{"level":"warn","msg":"warn msg"}"#;

    let output = cor()
        .arg("--color=never")
        .arg("--level=warn")
        .arg("--json")
        .write_stdin(input)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        !stdout.contains("debug"),
        "debug should be filtered in --json mode"
    );
    assert!(
        stdout.contains("warn msg"),
        "warn should pass in --json mode"
    );
    // Output should be valid JSON
    assert!(stdout.contains(r#""level":"warn""#));
}
