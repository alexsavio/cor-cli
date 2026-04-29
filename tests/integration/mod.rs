mod basic_pipe;
mod cli_flags;
mod color_control;
mod config_custom;
mod embedded_json;
mod level_filter;
mod mixed_input;
mod multiline;
mod streaming;

use assert_cmd::Command;

/// Shared helper: build a `cor` command with config isolation.
#[allow(deprecated)]
pub fn cor() -> Command {
    let mut cmd = Command::cargo_bin("cor").unwrap();
    cmd.env("XDG_CONFIG_HOME", "/tmp/cor-test-no-config");
    cmd
}
