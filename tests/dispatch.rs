//! Tests for runtime mode selection.
//!
//! `dispatch` is intentionally pure — it takes a slice of argv and a
//! snapshot of the environment, and returns a `Mode`. That makes every
//! branch testable without spawning processes or mocking `/proc`.

use rofi_rbw_rs::dispatch::{dispatch, is_pinentry_argv0, Env, Mode};

// -- fixtures --------------------------------------------------------------

fn args(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

fn env(tty: bool, parent: Option<&str>) -> Env {
    Env {
        stdin_is_tty: tty,
        parent_comm: parent.map(|s| s.to_string()),
    }
}

/// What a normal terminal invocation looks like: shell parent, stdin is tty.
fn shell_session() -> Env {
    env(true, Some("zsh"))
}

/// What `rbw` spawns pinentry with: piped stdin, parent is `rbw`.
fn rbw_pipe() -> Env {
    env(false, Some("rbw"))
}

/// Same, but `rbw` is running as the agent.
fn rbw_agent_pipe() -> Env {
    env(false, Some("rbw-agent"))
}

// -- flag dispatch ---------------------------------------------------------

#[test]
fn pinentry_flag_forces_pinentry_mode() {
    let a = args(&["rofi-rbw-rs", "--pinentry"]);
    assert_eq!(dispatch(&a, &shell_session()), Mode::Pinentry);
    assert_eq!(dispatch(&a, &rbw_pipe()), Mode::Pinentry);
}

#[test]
fn pinentry_clear_flag_forces_clear_mode() {
    let a = args(&["rofi-rbw-rs", "--pinentry-clear"]);
    assert_eq!(dispatch(&a, &shell_session()), Mode::PinentryClear);
    assert_eq!(dispatch(&a, &rbw_pipe()), Mode::PinentryClear);
}

#[test]
fn flag_wins_over_argv0() {
    // Invoked as `rbw-pinentry --pinentry-clear` — the clear flag should
    // take precedence over the argv[0]-implied pinentry mode.
    let a = args(&["rbw-pinentry", "--pinentry-clear"]);
    assert_eq!(dispatch(&a, &shell_session()), Mode::PinentryClear);
}

#[test]
fn flag_wins_over_parent_heuristic() {
    // Parent is `rbw`, stdin is a pipe — normally that means pinentry mode.
    // But an explicit `--pinentry-clear` must override.
    let a = args(&["rofi-rbw-rs", "--pinentry-clear"]);
    assert_eq!(dispatch(&a, &rbw_pipe()), Mode::PinentryClear);
}

// -- argv[0] dispatch ------------------------------------------------------

#[test]
fn argv0_bare_name_triggers_pinentry() {
    let a = args(&["rbw-pinentry"]);
    // Even with an otherwise-frontend-looking environment, the name wins.
    assert_eq!(dispatch(&a, &shell_session()), Mode::Pinentry);
}

#[test]
fn argv0_absolute_path_triggers_pinentry() {
    let a = args(&["/usr/bin/rbw-pinentry"]);
    assert_eq!(dispatch(&a, &shell_session()), Mode::Pinentry);
}

#[test]
fn argv0_alternate_alias_triggers_pinentry() {
    let a = args(&["/usr/local/bin/rofi-rbw-pinentry"]);
    assert_eq!(dispatch(&a, &shell_session()), Mode::Pinentry);
}

#[test]
fn argv0_different_name_does_not_trigger() {
    let a = args(&["/usr/bin/rofi-rbw-rs"]);
    assert_eq!(dispatch(&a, &shell_session()), Mode::Frontend);
}

#[test]
fn argv0_partial_match_does_not_trigger() {
    // `rbw-pinentry-helper` is not a recognised alias.
    let a = args(&["/usr/bin/rbw-pinentry-helper"]);
    assert_eq!(dispatch(&a, &shell_session()), Mode::Frontend);
}

#[test]
fn is_pinentry_argv0_direct() {
    assert!(is_pinentry_argv0("rbw-pinentry"));
    assert!(is_pinentry_argv0("/usr/bin/rbw-pinentry"));
    assert!(is_pinentry_argv0("rofi-rbw-pinentry"));
    assert!(is_pinentry_argv0("./rofi-rbw-pinentry"));

    assert!(!is_pinentry_argv0("rofi-rbw-rs"));
    assert!(!is_pinentry_argv0(""));
    assert!(!is_pinentry_argv0("/"));
    assert!(!is_pinentry_argv0("rbw-pinentry-helper"));
    assert!(!is_pinentry_argv0("rbw-pinentry.exe"));
}

// -- parent heuristic ------------------------------------------------------

#[test]
fn parent_rbw_with_pipe_triggers_pinentry() {
    let a = args(&["rofi-rbw-rs"]);
    assert_eq!(dispatch(&a, &rbw_pipe()), Mode::Pinentry);
}

#[test]
fn parent_rbw_agent_with_pipe_triggers_pinentry() {
    let a = args(&["rofi-rbw-rs"]);
    assert_eq!(dispatch(&a, &rbw_agent_pipe()), Mode::Pinentry);
}

#[test]
fn parent_rbw_but_stdin_is_tty_does_not_trigger() {
    // Someone ran `rbw` from a shell that inherited a TTY somehow, or is
    // manually poking at the binary. Either way, TTY stdin means we're
    // interactive — don't hijack that for pinentry.
    let a = args(&["rofi-rbw-rs"]);
    let e = env(true, Some("rbw"));
    assert_eq!(dispatch(&a, &e), Mode::Frontend);
}

#[test]
fn parent_shell_with_pipe_does_not_trigger() {
    // e.g. `echo hi | rofi-rbw-rs` from a shell — pipe but wrong parent.
    let a = args(&["rofi-rbw-rs"]);
    let e = env(false, Some("zsh"));
    assert_eq!(dispatch(&a, &e), Mode::Frontend);
}

#[test]
fn parent_unknown_with_pipe_does_not_trigger() {
    // Non-Linux, or PPid was 1, or /proc unreadable. Fail closed: normal mode.
    let a = args(&["rofi-rbw-rs"]);
    let e = env(false, None);
    assert_eq!(dispatch(&a, &e), Mode::Frontend);
}

#[test]
fn parent_bash_with_pipe_does_not_trigger() {
    let a = args(&["rofi-rbw-rs"]);
    let e = env(false, Some("bash"));
    assert_eq!(dispatch(&a, &e), Mode::Frontend);
}

#[test]
fn parent_rbw_prefix_does_not_trigger() {
    // `rbw-something-else` isn't `rbw` or `rbw-agent`.
    let a = args(&["rofi-rbw-rs"]);
    let e = env(false, Some("rbw-something"));
    assert_eq!(dispatch(&a, &e), Mode::Frontend);
}

// -- default path ----------------------------------------------------------

#[test]
fn normal_terminal_invocation_is_frontend() {
    let a = args(&["rofi-rbw-rs"]);
    assert_eq!(dispatch(&a, &shell_session()), Mode::Frontend);
}

#[test]
fn normal_terminal_invocation_with_flags_is_frontend() {
    let a = args(&["rofi-rbw-rs", "-a", "type", "-t", "password"]);
    assert_eq!(dispatch(&a, &shell_session()), Mode::Frontend);
}

#[test]
fn empty_argv_is_frontend() {
    let a: Vec<String> = Vec::new();
    assert_eq!(dispatch(&a, &shell_session()), Mode::Frontend);
}

#[test]
fn no_second_arg_is_frontend() {
    // Only the program name — no `--pinentry`, no flag.
    let a = args(&["rofi-rbw-rs"]);
    assert_eq!(dispatch(&a, &shell_session()), Mode::Frontend);
}

// -- precedence summary ----------------------------------------------------

#[test]
fn precedence_flag_beats_argv0_beats_parent() {
    // Flag present → flag wins regardless of anything else.
    let a = args(&["rbw-pinentry", "--pinentry-clear"]);
    assert_eq!(dispatch(&a, &rbw_pipe()), Mode::PinentryClear);

    // No flag, but argv[0] matches → argv[0] wins over parent heuristic.
    let a = args(&["rbw-pinentry"]);
    assert_eq!(dispatch(&a, &shell_session()), Mode::Pinentry);

    // No flag, no argv[0] match, parent is rbw → parent heuristic.
    let a = args(&["rofi-rbw-rs"]);
    assert_eq!(dispatch(&a, &rbw_pipe()), Mode::Pinentry);

    // Nothing matches → frontend.
    let a = args(&["rofi-rbw-rs"]);
    assert_eq!(dispatch(&a, &shell_session()), Mode::Frontend);
}
