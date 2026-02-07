//! Integration tests for color control: `NO_COLOR`, `FORCE_COLOR`, --color flag, `TERM`.

use assert_cmd::Command;

#[allow(deprecated)]
fn cor() -> Command {
    Command::cargo_bin("cor").unwrap()
}

#[test]
fn color_never_disables_ansi() {
    let input = r#"{"level":"info","msg":"hello"}"#;
    let output = cor()
        .arg("--color=never")
        .write_stdin(input)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    // No ANSI escape sequences
    assert!(
        !stdout.contains("\x1b["),
        "Should not contain ANSI escapes with --color=never"
    );
}

#[test]
fn color_always_enables_ansi() {
    let input = r#"{"level":"info","msg":"hello"}"#;
    let output = cor()
        .arg("--color=always")
        .write_stdin(input)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should contain ANSI escape sequences
    assert!(
        stdout.contains("\x1b["),
        "Should contain ANSI escapes with --color=always"
    );
}

#[test]
fn no_color_env_disables_colors() {
    let input = r#"{"level":"info","msg":"hello"}"#;
    let output = cor()
        .env("NO_COLOR", "1")
        .write_stdin(input)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Piped stdout + NO_COLOR → no colors
    assert!(
        !stdout.contains("\x1b["),
        "Should not contain ANSI escapes with NO_COLOR set"
    );
}

#[test]
fn color_always_overrides_no_color() {
    let input = r#"{"level":"info","msg":"hello"}"#;
    let output = cor()
        .arg("--color=always")
        .env("NO_COLOR", "1")
        .write_stdin(input)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    // --color=always overrides NO_COLOR
    assert!(
        stdout.contains("\x1b["),
        "--color=always should override NO_COLOR"
    );
}

#[test]
fn piped_stdout_disables_colors_by_default() {
    let input = r#"{"level":"info","msg":"hello"}"#;
    let output = cor().write_stdin(input).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    // When piped (not a TTY), auto mode should disable colors
    assert!(
        !stdout.contains("\x1b["),
        "Piped output should not have ANSI escapes in auto mode"
    );
}

#[test]
fn term_dumb_disables_colors() {
    let input = r#"{"level":"info","msg":"hello"}"#;
    let output = cor()
        .env("TERM", "dumb")
        .write_stdin(input)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("\x1b["),
        "TERM=dumb should disable colors in auto mode"
    );
}

#[test]
fn color_never_overrides_force_color() {
    let input = r#"{"level":"info","msg":"hello"}"#;
    let output = cor()
        .arg("--color=never")
        .env("FORCE_COLOR", "1")
        .write_stdin(input)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("\x1b["),
        "--color=never should override FORCE_COLOR"
    );
}
