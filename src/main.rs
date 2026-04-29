use std::fs::File;
use std::io::{self, BufRead, BufReader, LineWriter, Write};
use std::path::Path;
use std::process::ExitCode;

use clap::Parser;

use cor::cli::{Cli, ColorMode};
use cor::config::Config;
use cor::formatter::{format_line, format_line_parsed};
use cor::parser::{self, LineKind};

/// Maximum number of continuation lines to buffer when reassembling
/// multi-line JSON (e.g., exception tracebacks with raw newlines).
///
/// This limit prevents unbounded memory growth when a line starts with `{"`
/// but never forms valid JSON. 200 lines accommodates most real-world
/// tracebacks while bounding worst-case memory to ~200KB (assuming 1KB/line).
const MAX_JSON_CONTINUATION_LINES: usize = 200;

/// Convert an I/O result to an optional exit code.
///
/// - `Ok(())` → `None` (continue processing)
/// - `BrokenPipe` → `Some(SUCCESS)` (graceful termination)
/// - Other errors → `Some(2)` after printing error message
#[inline]
fn check_write_result(result: io::Result<()>, context: &str) -> Option<ExitCode> {
    match result {
        Ok(()) => None,
        Err(e) if e.kind() == io::ErrorKind::BrokenPipe => Some(ExitCode::SUCCESS),
        Err(e) => {
            eprintln!("cor: {context}: {e}");
            Some(ExitCode::from(2))
        }
    }
}

/// Write a formatted line with line gap, returning early exit code on error.
///
/// Batches the entry and its trailing blank lines into a single `write!`
/// call so `LineWriter` only flushes once per entry (on the final newline)
/// rather than `1 + line_gap` times. Keeps streaming responsive without
/// paying per-gap syscalls in batch mode.
#[inline]
fn write_entry(
    writer: &mut LineWriter<io::StdoutLock<'_>>,
    line_buf: &str,
    line_gap: usize,
) -> Option<ExitCode> {
    // One '\n' to terminate the entry + `line_gap` blank-line newlines.
    let trailing = "\n".repeat(1 + line_gap);
    check_write_result(write!(writer, "{line_buf}{trailing}"), "write error")
}

fn main() -> ExitCode {
    // Reset SIGPIPE to default behavior so upstream writers get a clean
    // SIGPIPE signal instead of a BrokenPipeError when cor exits early.
    reset_sigpipe();

    let cli = Cli::parse();

    // Handle --completions: generate and exit
    if let Some(shell) = cli.completions {
        let mut cmd = <Cli as clap::CommandFactory>::command();
        clap_complete::generate(shell, &mut cmd, "cor", &mut io::stdout());
        return ExitCode::SUCCESS;
    }

    let config = match Config::from_cli(&cli) {
        Ok(config) => config,
        Err(e) => {
            eprintln!("cor: {e}");
            return ExitCode::from(1);
        }
    };

    match config.color_mode {
        ColorMode::Always => owo_colors::set_override(true),
        ColorMode::Never => owo_colors::set_override(false),
        ColorMode::Auto => {} // owo-colors auto-detects via supports-color
    }

    let stdout = io::stdout();
    // LineWriter flushes on every newline so streaming inputs (e.g.
    // `kubectl logs -f`) print immediately instead of waiting for EOF
    // or for a block buffer to fill. See issue #3.
    //
    // Use an 8 KiB capacity to match the previous `BufWriter::new` default
    // so long formatted lines (many fields, large values) still get
    // coalesced into a single write before the trailing newline triggers
    // the flush. `LineWriter::new` would default to 1 KiB.
    let mut writer = LineWriter::with_capacity(8 * 1024, stdout.lock());
    let mut had_error = false;

    if cli.files.is_empty() {
        // No files: read from stdin (original behavior)
        let stdin = io::stdin();
        let exit = process_lines(stdin.lock().lines(), &config, &mut writer);
        if let Some(code) = exit {
            return code;
        }
    } else {
        for path in &cli.files {
            let exit = if path == Path::new("-") {
                let stdin = io::stdin();
                process_lines(stdin.lock().lines(), &config, &mut writer)
            } else {
                match File::open(path) {
                    Ok(file) => {
                        let reader = BufReader::new(file);
                        process_lines(reader.lines(), &config, &mut writer)
                    }
                    Err(e) => {
                        eprintln!("cor: {}: {e}", path.display());
                        had_error = true;
                        continue;
                    }
                }
            };
            if let Some(code) = exit {
                return code;
            }
        }
    }

    if let Some(code) = check_write_result(writer.flush(), "flush error") {
        return code;
    }

    if had_error {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

/// Process all input lines, handling single-line and multi-line JSON reassembly.
///
/// Returns `Some(ExitCode)` for early termination (errors / broken pipe),
/// or `None` when all input has been processed normally.
fn process_lines(
    mut lines_iter: impl Iterator<Item = io::Result<String>>,
    config: &Config,
    writer: &mut LineWriter<io::StdoutLock<'_>>,
) -> Option<ExitCode> {
    let mut line_buf = String::new();

    while let Some(line_result) = lines_iter.next() {
        let line = match line_result {
            Ok(l) => l,
            Err(e) if e.kind() == io::ErrorKind::InvalidData => continue,
            Err(e) => {
                eprintln!("cor: read error: {e}");
                return Some(ExitCode::from(2));
            }
        };

        // Try normal single-line parsing first.
        let parsed = parser::parse_line(&line, config);

        match parsed {
            LineKind::Raw(_) if might_start_json(&line) => {
                // The line contains '{' but failed to parse — may be split
                // across multiple lines due to raw newlines in JSON strings.
                let mut buffer = line;
                let mut assembled = false;

                for _ in 0..MAX_JSON_CONTINUATION_LINES {
                    let next = match lines_iter.next() {
                        Some(Ok(l)) => l,
                        Some(Err(e)) if e.kind() == io::ErrorKind::InvalidData => continue,
                        _ => break,
                    };

                    buffer.push('\n');
                    buffer.push_str(&next);

                    // Sanitize raw newlines inside JSON strings, then re-parse.
                    let sanitized = parser::sanitize_json_newlines(&buffer);
                    let re_parsed = parser::parse_line(&sanitized, config);

                    if !matches!(re_parsed, LineKind::Raw(_)) {
                        // Successfully assembled — format the sanitized version.
                        line_buf.clear();
                        format_line_parsed(re_parsed, &sanitized, config, &mut line_buf);
                        assembled = true;
                        break;
                    }
                }

                if !assembled {
                    // Could not reassemble — output each buffered line as raw.
                    for raw_line in buffer.split('\n') {
                        line_buf.clear();
                        format_line(raw_line, config, &mut line_buf);
                        if !line_buf.is_empty()
                            && let exit @ Some(_) = write_entry(writer, &line_buf, config.line_gap)
                        {
                            return exit;
                        }
                    }
                    continue;
                }
            }
            _ => {
                line_buf.clear();
                format_line_parsed(parsed, &line, config, &mut line_buf);
            }
        }

        // Filtered-out lines produce an empty buffer — skip them.
        if line_buf.is_empty() {
            continue;
        }

        if let exit @ Some(_) = write_entry(writer, &line_buf, config.line_gap) {
            return exit;
        }
    }

    None
}

/// Check if a line might be the start of an incomplete JSON object.
///
/// Returns `true` if the line contains `{"` which is a strong indicator
/// of a JSON object start. This avoids false positives from lines that
/// contain stray `{` characters (e.g., code snippets).
fn might_start_json(line: &str) -> bool {
    let trimmed = line.trim();
    if let Some(brace_pos) = trimmed.find('{') {
        let after_brace = &trimmed[brace_pos + 1..];
        after_brace.trim_start().starts_with('"')
    } else {
        false
    }
}

/// Reset SIGPIPE to the default (terminate) behavior.
///
/// By default, Rust ignores SIGPIPE to surface `BrokenPipe` I/O errors.
/// For a CLI filter like `cor`, this causes the *upstream* writer (e.g. a
/// Python process) to receive a `BrokenPipeError` when `cor` exits.
/// Restoring `SIG_DFL` lets the OS handle the signal normally.
#[cfg(unix)]
fn reset_sigpipe() {
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }
}

#[cfg(not(unix))]
fn reset_sigpipe() {}
