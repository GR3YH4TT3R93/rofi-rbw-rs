use std::io::Write;
use std::process::{Command, Output, Stdio};

use anyhow::{Context, Result};
use which::which;

pub fn is_installed(name: &str) -> bool {
    which(name).is_ok()
}

pub fn is_wayland() -> bool {
    std::env::var_os("WAYLAND_DISPLAY").is_some()
}

/// Spawn a subprocess, feed it `input` on stdin, capture stdout.
/// stderr is inherited so the user sees e.g. rofi's own errors.
pub fn run_with_stdin(cmd: &mut Command, input: &str) -> Result<Output> {
    cmd.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit());

    let mut child = cmd.spawn().context("failed to spawn subprocess")?;
    {
        let mut stdin = child.stdin.take().expect("stdin was piped");
        // The child may exit before reading everything (user cancel) — ignore.
        let _ = stdin.write_all(input.as_bytes());
    }
    child.wait_with_output().context("subprocess failed")
}
