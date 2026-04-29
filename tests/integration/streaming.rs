//! Regression tests for streaming stdin (issue #3).
//!
//! When `cor` is fed by a long-running producer (e.g. `kubectl logs -f`),
//! each parsed line must be flushed to stdout as soon as it is produced,
//! not held in an internal block buffer until EOF. These tests spawn `cor`
//! with piped stdin/stdout, write a single JSON line, and assert the
//! formatted output appears before stdin is closed.

use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

/// Spawn `cor`, write one line to stdin, and read the first line from stdout
/// within `timeout`. Returns `Some(line)` if a line was produced in time,
/// `None` if the read timed out (i.e. cor was buffering).
fn first_line_within(input: &str, timeout: Duration) -> Option<String> {
    let bin = assert_cmd::cargo::cargo_bin!("cor");
    let mut child = Command::new(bin)
        .arg("--color=never")
        .env("XDG_CONFIG_HOME", "/tmp/cor-test-no-config")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn cor");

    {
        let stdin = child.stdin.as_mut().expect("stdin pipe");
        stdin.write_all(input.as_bytes()).unwrap();
        stdin.write_all(b"\n").unwrap();
        stdin.flush().unwrap();
    }
    // Keep stdin open: the goal is to prove output flushes before EOF.

    let stdout = child.stdout.take().expect("stdout pipe");
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let mut reader = BufReader::new(stdout);
        let mut line = String::new();
        let _ = reader.read_line(&mut line);
        let _ = tx.send(line);
    });

    let result = rx.recv_timeout(timeout).ok();

    // Close stdin and wait for the child to exit so we don't leak processes.
    drop(child.stdin.take());
    let _ = child.wait();

    result.filter(|s| !s.is_empty())
}

#[test]
fn json_line_flushed_before_stdin_closes() {
    // The classic kubectl-logs-f case: a single JSON record arrives, and
    // the user expects to see it formatted immediately.
    let input = r#"{"level":"info","msg":"streaming","port":8080}"#;
    let line = first_line_within(input, Duration::from_secs(5))
        .expect("cor did not flush a line before timing out (issue #3 regression)");

    assert!(
        line.contains("INFO"),
        "expected formatted output, got: {line:?}"
    );
    assert!(
        line.contains("streaming"),
        "expected msg in output, got: {line:?}"
    );
}

#[test]
fn raw_passthrough_line_flushed_before_stdin_closes() {
    // Non-JSON lines pass through unchanged; they too must flush per-line.
    let input = "plain text line that is not json";
    let line = first_line_within(input, Duration::from_secs(5))
        .expect("cor did not flush a raw passthrough line before timing out");

    assert!(
        line.contains("plain text line that is not json"),
        "expected raw passthrough, got: {line:?}"
    );
}
