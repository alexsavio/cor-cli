use std::io::{self, BufRead, BufWriter, IsTerminal, Write};
use std::process::ExitCode;

use clap::Parser;

use cor::cli::{Cli, ColorMode};
use cor::config::Config;
use cor::formatter::{format_line, format_line_parsed};
use cor::parser::{self, LineKind};

/// Maximum number of continuation lines to buffer when reassembling
/// multi-line JSON (e.g., exception tracebacks with raw newlines).
const MAX_JSON_CONTINUATION_LINES: usize = 200;

fn main() -> ExitCode {
    // Reset SIGPIPE to default behavior so upstream writers get a clean
    // SIGPIPE signal instead of a BrokenPipeError when cor exits early.
    reset_sigpipe();

    let cli = Cli::parse();

    let config = match Config::from_cli(&cli) {
        Ok(config) => config,
        Err(e) => {
            eprintln!("cor: {e}");
            return ExitCode::from(1);
        }
    };

    let use_color = resolve_color_mode(config.color_mode);

    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut writer = BufWriter::new(stdout.lock());

    let exit = process_lines(stdin.lock().lines(), &config, use_color, &mut writer);
    if let Some(code) = exit {
        return code;
    }

    if let Err(e) = writer.flush() {
        if e.kind() == io::ErrorKind::BrokenPipe {
            return ExitCode::SUCCESS;
        }
        eprintln!("cor: flush error: {e}");
        return ExitCode::from(2);
    }

    ExitCode::SUCCESS
}

/// Process all input lines, handling single-line and multi-line JSON reassembly.
///
/// Returns `Some(ExitCode)` for early termination (errors / broken pipe),
/// or `None` when all input has been processed normally.
fn process_lines(
    mut lines_iter: impl Iterator<Item = io::Result<String>>,
    config: &Config,
    use_color: bool,
    writer: &mut BufWriter<io::StdoutLock<'_>>,
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
            LineKind::Raw if might_start_json(&line) => {
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

                    if !matches!(re_parsed, LineKind::Raw) {
                        // Successfully assembled — format the sanitized version.
                        line_buf.clear();
                        format_line_parsed(re_parsed, &sanitized, config, use_color, &mut line_buf);
                        assembled = true;
                        break;
                    }
                }

                if !assembled {
                    // Could not reassemble — output each buffered line as raw.
                    line_buf.clear();
                    for raw_line in buffer.split('\n') {
                        line_buf.clear();
                        format_line(raw_line, config, use_color, &mut line_buf);
                        if !line_buf.is_empty()
                            && let Err(e) = writeln!(writer, "{line_buf}")
                        {
                            if e.kind() == io::ErrorKind::BrokenPipe {
                                return Some(ExitCode::SUCCESS);
                            }
                            eprintln!("cor: write error: {e}");
                            return Some(ExitCode::from(2));
                        }
                    }
                    continue;
                }
            }
            _ => {
                line_buf.clear();
                format_line_parsed(parsed, &line, config, use_color, &mut line_buf);
            }
        }

        // Filtered-out lines produce an empty buffer — skip them.
        if line_buf.is_empty() {
            continue;
        }

        if let Err(e) = writeln!(writer, "{line_buf}") {
            if e.kind() == io::ErrorKind::BrokenPipe {
                return Some(ExitCode::SUCCESS);
            }
            eprintln!("cor: write error: {e}");
            return Some(ExitCode::from(2));
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

fn resolve_color_mode(mode: ColorMode) -> bool {
    match mode {
        ColorMode::Always => true,
        ColorMode::Never => false,
        ColorMode::Auto => {
            let stdout = io::stdout();
            if !stdout.is_terminal() {
                return false;
            }
            if std::env::var_os("NO_COLOR").is_some_and(|v| !v.is_empty()) {
                return false;
            }
            if std::env::var("TERM").is_ok_and(|v| v == "dumb") {
                return false;
            }
            if std::env::var_os("FORCE_COLOR").is_some_and(|v| !v.is_empty()) {
                return true;
            }
            true
        }
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
