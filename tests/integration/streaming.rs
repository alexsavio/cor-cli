//! Regression tests for streaming stdin (issue #3).
//!
//! When `cor` is fed by a long-running producer (e.g. `kubectl logs -f`),
//! each parsed line must be flushed to stdout as soon as it is produced,
//! not held in an internal block buffer until EOF. These tests spawn `cor`
//! with piped stdin/stdout, write a single JSON line, and assert the
//! formatted output appears *before* stdin is closed.

use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

/// Spawn `cor`, write one line to stdin, and read the first stdout line
/// within `timeout` while keeping stdin open. Returns `Some(line)` if a
/// line was produced in time, `None` if the read timed out (i.e. cor was
/// buffering and never flushed).
fn first_line_within(input: &str, timeout: Duration) -> Option<String> {
    let mut child = spawn_cor(&["--color=never"]);
    write_input(&mut child, input);

    let stdout = child.stdout.take().expect("stdout pipe");
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let mut reader = BufReader::new(stdout);
        let mut line = String::new();
        let _ = reader.read_line(&mut line);
        let _ = tx.send(line);
    });

    let result = rx.recv_timeout(timeout).ok();

    drop(child.stdin.take());
    let _ = child.wait();

    result.filter(|s| !s.is_empty())
}

/// Spawn `cor`, write `input` + newline, give it `timeout` to flush at
/// least `min_bytes` of formatted output **while stdin is still open**,
/// and return whatever was collected. If the buffer is still under
/// `min_bytes` after `timeout`, returns `None` — the output is being
/// held back, which is exactly the issue #3 regression we guard against.
fn output_within(
    input: &str,
    args: &[&str],
    min_bytes: usize,
    timeout: Duration,
) -> Option<String> {
    let mut child = spawn_cor(args);
    write_input(&mut child, input);

    let mut stdout = child.stdout.take().expect("stdout pipe");
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let mut buf = Vec::new();
        let mut chunk = [0u8; 4096];
        loop {
            match stdout.read(&mut chunk) {
                Ok(0) | Err(_) => break, // EOF or read error
                Ok(n) => {
                    buf.extend_from_slice(&chunk[..n]);
                    if buf.len() >= min_bytes && tx.send(buf.clone()).is_err() {
                        break;
                    }
                }
            }
        }
        let _ = tx.send(buf);
    });

    let result = rx.recv_timeout(timeout).ok();

    drop(child.stdin.take());
    let _ = child.wait();

    result
        .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
        .filter(|s| s.len() >= min_bytes)
}

fn spawn_cor(args: &[&str]) -> std::process::Child {
    let bin = assert_cmd::cargo::cargo_bin!("cor");
    Command::new(bin)
        .args(args)
        .env("XDG_CONFIG_HOME", "/tmp/cor-test-no-config")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn cor")
}

fn write_input(child: &mut std::process::Child, input: &str) {
    let stdin = child.stdin.as_mut().expect("stdin pipe");
    stdin
        .write_all(input.as_bytes())
        .expect("write input to cor stdin");
    stdin
        .write_all(b"\n")
        .expect("write trailing newline to cor stdin");
    stdin.flush().expect("flush cor stdin");
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
fn long_2kib_line_flushed_before_stdin_closes() {
    // For writes larger than `LineWriter`'s internal buffer the writer
    // flushes the existing buffer first and then writes the overflow
    // straight through to the inner `Stdout`; data is never lost, but the
    // buffering benefit goes away. Bumping the capacity to 8 KiB keeps
    // long formatted entries inside the buffer until the trailing newline
    // triggers a single flush. The fixture below comfortably exceeds 2
    // KiB because the `data` value alone is 2 KiB and survives truncation
    // thanks to `--max-field-length 0`. Output spans multiple lines, so
    // we read bytes (not just the first line) until at least 2 KiB lands.
    let big = "x".repeat(2048);
    let input = format!(r#"{{"level":"info","msg":"big","data":"{big}"}}"#);
    let output = output_within(
        &input,
        &["--color=never", "--max-field-length", "0"],
        2048,
        Duration::from_secs(5),
    )
    .expect("cor did not flush a >2KiB entry before timing out (issue #3 regression)");

    assert!(
        output.contains("INFO"),
        "expected formatted level, got len={}",
        output.len()
    );
    assert!(
        output.contains(&big),
        "expected full 2KiB value in output, got len={}",
        output.len()
    );
}

#[test]
fn multiline_json_reassembly_flushed_before_stdin_closes() {
    // Multi-line JSON (raw newlines inside a string) is reassembled in
    // `process_lines` and then handed to the same `write_entry` path as
    // single-line JSON. This guards the reassembly branch against any
    // future change that adds its own buffering.
    let input = "{\"level\":\"error\",\"msg\":\"boom\",\"trace\":\"line1\nline2\nline3\"}";
    let output = output_within(
        input,
        &["--color=never"],
        // Enough bytes to require both the level line and a follow-up
        // field line to be flushed.
        40,
        Duration::from_secs(5),
    )
    .expect("cor did not flush a reassembled multi-line entry before timing out");

    assert!(output.contains("ERROR"), "expected level: {output:?}");
    assert!(output.contains("boom"), "expected msg: {output:?}");
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
