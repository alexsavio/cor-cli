//! Integration tests for User Story 3 - custom keys, field filtering, config file.

use assert_cmd::Command;
use predicates::prelude::*;
use std::io::Write;

#[allow(deprecated)]
fn cor() -> Command {
    Command::cargo_bin("cor").unwrap()
}

#[test]
fn custom_message_key() {
    let input = r#"{"level":"info","event":"something happened","port":8080}"#;
    cor()
        .arg("--color=never")
        .arg("--message-key=event")
        .write_stdin(input)
        .assert()
        .success()
        .stdout(predicate::str::contains("INFO"))
        .stdout(predicate::str::contains("something happened"))
        .stdout(predicate::str::contains("port: 8080"));
}

#[test]
fn custom_level_key() {
    let input = r#"{"severity":"warn","msg":"disk low"}"#;
    cor()
        .arg("--color=never")
        .arg("--level-key=severity")
        .write_stdin(input)
        .assert()
        .success()
        .stdout(predicate::str::contains("WARN"))
        .stdout(predicate::str::contains("disk low"));
}

#[test]
fn custom_timestamp_key() {
    let input = r#"{"datetime":"2026-01-15T10:30:00Z","level":"info","msg":"hello"}"#;
    cor()
        .arg("--color=never")
        .arg("--timestamp-key=datetime")
        .write_stdin(input)
        .assert()
        .success()
        .stdout(predicate::str::contains("10:30:00.000"))
        .stdout(predicate::str::contains("INFO"))
        .stdout(predicate::str::contains("hello"));
}

#[test]
fn include_fields_only_shows_specified() {
    let input = r#"{"level":"info","msg":"test","port":8080,"host":"localhost","pid":1234}"#;
    let output = cor()
        .arg("--color=never")
        .arg("--include-fields=port")
        .write_stdin(input)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("port: 8080"),
        "included field should appear"
    );
    assert!(
        !stdout.contains("host:"),
        "non-included field should be hidden"
    );
    assert!(
        !stdout.contains("pid:"),
        "non-included field should be hidden"
    );
}

#[test]
fn exclude_fields_hides_specified() {
    let input = r#"{"level":"info","msg":"test","port":8080,"host":"localhost","pid":1234}"#;
    let output = cor()
        .arg("--color=never")
        .arg("--exclude-fields=pid,host")
        .write_stdin(input)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("port: 8080"),
        "non-excluded field should appear"
    );
    assert!(!stdout.contains("host:"), "excluded field should be hidden");
    assert!(!stdout.contains("pid:"), "excluded field should be hidden");
}

#[test]
fn include_and_exclude_mutually_exclusive() {
    let input = r#"{"level":"info","msg":"test"}"#;
    cor()
        .arg("--include-fields=port")
        .arg("--exclude-fields=pid")
        .write_stdin(input)
        .assert()
        .failure()
        .code(2); // clap uses exit code 2 for argument errors
}

#[test]
fn json_output_mode() {
    let input = r#"{"level":"info","msg":"hello","port":8080}"#;
    cor()
        .arg("--color=never")
        .arg("--json")
        .write_stdin(input)
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""level":"info""#))
        .stdout(predicate::str::contains(r#""msg":"hello""#));
}

#[test]
fn json_output_suppresses_non_json() {
    let input = "plain text\n{\"level\":\"info\",\"msg\":\"hello\"}\n";
    let output = cor()
        .arg("--color=never")
        .arg("--json")
        .write_stdin(input)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("plain text"),
        "Non-JSON lines should be suppressed in --json mode"
    );
    assert!(stdout.contains(r#""level":"info""#));
}

#[test]
fn custom_max_field_length() {
    let long_val = "x".repeat(50);
    let input = format!(r#"{{"level":"info","msg":"test","data":"{long_val}"}}"#);
    let output = cor()
        .arg("--color=never")
        .arg("--max-field-length=20")
        .write_stdin(input)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains('…'),
        "Value should be truncated at custom max length"
    );
    assert!(!stdout.contains(&long_val), "Full value should not appear");
}

#[test]
fn config_file_custom_keys() {
    let config_content = r#"
[keys]
message = "event"
level = "sev"
"#;
    let mut config_file = tempfile::NamedTempFile::new().unwrap();
    config_file.write_all(config_content.as_bytes()).unwrap();

    let input = r#"{"sev":"warn","event":"disk full","disk":"/dev/sda1"}"#;
    cor()
        .arg("--color=never")
        .arg(format!("--config={}", config_file.path().display()))
        .write_stdin(input)
        .assert()
        .success()
        .stdout(predicate::str::contains("WARN"))
        .stdout(predicate::str::contains("disk full"));
}

#[test]
fn config_file_custom_level_aliases() {
    let config_content = r#"
[levels]
"verbose" = "debug"
"critical" = "fatal"
"#;
    let mut config_file = tempfile::NamedTempFile::new().unwrap();
    config_file.write_all(config_content.as_bytes()).unwrap();

    let input = r#"{"level":"verbose","msg":"detailed info"}
{"level":"critical","msg":"system failure"}"#;
    let output = cor()
        .arg("--color=never")
        .arg(format!("--config={}", config_file.path().display()))
        .write_stdin(input)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("DEBUG"), "verbose should map to DEBUG");
    assert!(stdout.contains("FATAL"), "critical should map to FATAL");
}

#[test]
fn cli_overrides_config_file() {
    let config_content = r#"
[keys]
message = "event"
"#;
    let mut config_file = tempfile::NamedTempFile::new().unwrap();
    config_file.write_all(config_content.as_bytes()).unwrap();

    // CLI --message-key overrides config file
    let input = r#"{"body":"from body","event":"from event"}"#;
    cor()
        .arg("--color=never")
        .arg(format!("--config={}", config_file.path().display()))
        .arg("--message-key=body")
        .write_stdin(input)
        .assert()
        .success()
        .stdout(predicate::str::contains("from body"));
}
