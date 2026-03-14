//! Integration tests for multi-line JSON reassembly edge cases.

use super::cor;

#[test]
fn failed_reassembly_emits_all_lines_as_raw() {
    // A line starting with {"  but never forming valid JSON.
    // Followed by non-JSON lines that can't complete it.
    let input = r#"{"incomplete json that never closes
line two
line three
line four
line five
{"level":"info","msg":"next valid line"}"#;

    let output = cor()
        .arg("--color=never")
        .write_stdin(input)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);

    // All buffered lines should appear as raw text
    assert!(
        stdout.contains(r#"{"incomplete json that never closes"#),
        "First line should appear as raw.\nGot: {stdout}"
    );
    assert!(
        stdout.contains("line two"),
        "Buffered lines should appear as raw.\nGot: {stdout}"
    );
    assert!(
        stdout.contains("line three"),
        "Buffered lines should appear as raw.\nGot: {stdout}"
    );

    // The valid JSON line after the failed buffer should still be formatted
    assert!(
        stdout.contains("INFO"),
        "Valid JSON after failed reassembly should be formatted.\nGot: {stdout}"
    );
    assert!(
        stdout.contains("next valid line"),
        "Valid JSON message should be extracted.\nGot: {stdout}"
    );
}

#[test]
fn eof_during_reassembly_emits_buffer_as_raw() {
    // Input ends while the reassembly buffer is still open.
    let input = r#"{"incomplete json
that never closes"#;

    let output = cor()
        .arg("--color=never")
        .write_stdin(input)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Both lines should appear as raw text
    assert!(
        stdout.contains(r#"{"incomplete json"#),
        "First line should appear as raw on EOF.\nGot: {stdout}"
    );
    assert!(
        stdout.contains("that never closes"),
        "Second line should appear as raw on EOF.\nGot: {stdout}"
    );
}

#[test]
fn code_snippet_with_brace_not_json_treated_as_raw() {
    // A line like a code snippet that starts with { but isn't JSON
    let input = "{ this is a code snippet, not json }\nnext line";

    let output = cor()
        .arg("--color=never")
        .write_stdin(input)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Since `{ this...` doesn't have `{"` pattern, it should NOT trigger
    // multi-line reassembly at all — just pass through as raw immediately
    assert!(
        stdout.contains("{ this is a code snippet, not json }"),
        "Code snippet should pass through as raw.\nGot: {stdout}"
    );
    assert!(
        stdout.contains("next line"),
        "Following line should pass through.\nGot: {stdout}"
    );
}

#[test]
fn successful_reassembly_after_two_lines() {
    // JSON split across exactly 2 lines — should successfully reassemble.
    let input =
        b"{\"level\":\"warn\",\"msg\":\"split\nline\"}\n{\"level\":\"info\",\"msg\":\"after\"}\n";

    let output = cor()
        .arg("--color=never")
        .write_stdin(input.to_vec())
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains("WARN"),
        "Reassembled JSON should be formatted.\nGot: {stdout}"
    );
    assert!(
        stdout.contains("split"),
        "Message from reassembled JSON should appear.\nGot: {stdout}"
    );
    assert!(
        stdout.contains("INFO"),
        "Subsequent line should also be formatted.\nGot: {stdout}"
    );
}
