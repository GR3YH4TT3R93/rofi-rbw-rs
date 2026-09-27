//! Runtime mode selection for the multi-call binary.
//!
//! `rofi-rbw-rs` serves two roles: the interactive rofi frontend, and the
//! pinentry server that `rbw` execs during vault unlock. The role is chosen
//! at runtime without requiring a wrapper script or a `--flag` in `rbw`'s
//! config, using three signals in priority order:
//!
//! 1. `--pinentry` / `--pinentry-clear` on the command line (test/debug).
//! 2. `argv[0]` basename (`rbw-pinentry` — legacy hardlink path).
//! 3. Parent-process heuristic: stdin is a pipe and the parent is `rbw`
//!    or `rbw-agent`. That's the situation when `rbw` spawns its pinentry.
//!
//! The environment-dependent parts (`stdin is a tty?`, `parent comm name`)
//! are read into an [`Env`] struct so [`dispatch`] itself is a pure
//! function and fully testable.

use std::io::IsTerminal;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Run the Assuan pinentry server on stdin/stdout.
    Pinentry,
    /// Delete the stored master password from the keyring.
    PinentryClear,
    /// Normal interactive frontend.
    Frontend,
}

/// Everything [`dispatch`] needs to know about the process environment.
#[derive(Debug, Clone)]
pub struct Env {
    /// True when stdin is a terminal (interactive session).
    pub stdin_is_tty: bool,
    /// Basename of the parent process from `/proc/<ppid>/comm`, or `None`
    /// if it can't be determined (non-Linux, PPid=1, permission denied).
    pub parent_comm: Option<String>,
}

impl Env {
    /// Snapshot the current process's environment.
    pub fn detect() -> Self {
        Self {
            stdin_is_tty: std::io::stdin().is_terminal(),
            parent_comm: read_parent_comm(),
        }
    }
}

/// Decide which mode to run in. Pure function; all environment probing
/// happens in [`Env::detect`].
pub fn dispatch(args: &[String], env: &Env) -> Mode {
    if args.get(1).map(String::as_str) == Some("--pinentry") {
        return Mode::Pinentry;
    }
    if args.get(1).map(String::as_str) == Some("--pinentry-clear") {
        return Mode::PinentryClear;
    }

    let argv0 = args.first().map(String::as_str).unwrap_or("");
    if is_pinentry_argv0(argv0) {
        return Mode::Pinentry;
    }

    if !env.stdin_is_tty {
        if let Some(comm) = env.parent_comm.as_deref() {
            if comm == "rbw" || comm == "rbw-agent" {
                return Mode::Pinentry;
            }
        }
    }

    Mode::Frontend
}

/// True if `argv[0]`'s basename is one of the recognised pinentry aliases.
pub fn is_pinentry_argv0(argv0: &str) -> bool {
    let name = Path::new(argv0)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("");
    matches!(name, "rbw-pinentry" | "rofi-rbw-pinentry")
}

#[cfg(target_os = "linux")]
fn read_parent_comm() -> Option<String> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    let ppid: u32 = status
        .lines()
        .find_map(|l| l.strip_prefix("PPid:"))
        .and_then(|s| s.trim().parse().ok())?;
    if ppid <= 1 {
        return None;
    }
    std::fs::read_to_string(format!("/proc/{ppid}/comm"))
        .ok()
        .map(|s| s.trim().to_string())
}

#[cfg(not(target_os = "linux"))]
fn read_parent_comm() -> Option<String> {
    None
}
