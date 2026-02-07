use std::io::{self, BufRead, BufWriter, IsTerminal, Write};
use std::process::ExitCode;

use clap::Parser;

use cor::cli::{Cli, ColorMode};
use cor::config::Config;
use cor::formatter::format_line;

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
    let mut line_buf = String::new();

    let reader = stdin.lock();
    for line_result in reader.lines() {
        let line = match line_result {
            Ok(l) => l,
            Err(e) if e.kind() == io::ErrorKind::InvalidData => continue,
            Err(e) => {
                eprintln!("cor: read error: {e}");
                return ExitCode::from(2);
            }
        };

        line_buf.clear();
        format_line(&line, &config, use_color, &mut line_buf);

        // Filtered-out lines produce an empty buffer — skip them.
        if line_buf.is_empty() {
            continue;
        }

        if let Err(e) = writeln!(writer, "{line_buf}") {
            if e.kind() == io::ErrorKind::BrokenPipe {
                return ExitCode::SUCCESS;
            }
            eprintln!("cor: write error: {e}");
            return ExitCode::from(2);
        }
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
