//! Config file loading, mirroring `configargparse`'s `DefaultConfigFileParser`
//! format and `default_config_files` semantics.
//!
//! The Python side does NOT define an explicit `-c/--config` flag, so neither
//! do we. Only the standard locations from `paths::config_file_locations()`.
//!
//! Format supported (matches configargparse's default parser):
//!   * key = value / key : value / key value
//!   * keys may be written with `--`, `-`, or no prefix
//!   * bare key → treated as boolean `true`
//!   * `#` and `;` at start of line start a comment
//!   * `[section]` headers are ignored
//!   * line continuation via trailing `\`
//!   * values are split with shlex, so quoting works

use std::fs;
use std::path::Path;

use crate::paths::config_file_locations;

/// Build an argv fragment from every existing config file, in an order
/// that makes user config win over system config under clap's last-wins
/// semantics.
///
/// `config_file_locations()` returns `[user, system1, system2, ...]`. We
/// iterate in reverse so user config's argv fragment is emitted *last*,
/// which clap resolves to "user wins".
pub fn build_config_argv() -> Vec<String> {
    let mut argv = Vec::new();
    for path in config_file_locations().into_iter().rev() {
        if path.is_file() {
            argv.extend(parse_file(&path));
        }
    }
    argv
}

/// Parse a single config file into an argv fragment. Missing or unreadable
/// files yield an empty fragment (configargparse silently skips them).
fn parse_file(path: &Path) -> Vec<String> {
    match fs::read_to_string(path) {
        Ok(text) => parse_content(&text),
        Err(_) => Vec::new(),
    }
}

pub fn parse_content(text: &str) -> Vec<String> {
    let mut argv = Vec::new();
    let mut logical = String::new();

    for raw in text.lines() {
        // Line continuation: `\` at end of line joins with the next.
        if let Some(stripped) = raw.strip_suffix('\\') {
            logical.push_str(stripped);
            continue;
        }
        logical.push_str(raw);
        let line = std::mem::take(&mut logical);

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        // Comments: `#` or `;` start-of-line only (matches configargparse).
        if trimmed.starts_with('#') || trimmed.starts_with(';') {
            continue;
        }
        // Section headers — the rofi-rbw config is flat, so skip them.
        if trimmed.starts_with('[') {
            continue;
        }

        let (key, value) = split_key_value(trimmed);
        let (prefix, name) = split_dashes(key.trim());
        if name.is_empty() {
            continue;
        }

        // Bare key → boolean flag on.
        if value.is_empty() {
            argv.push(format!("{prefix}{name}"));
            continue;
        }

        let value_trimmed = value.trim();

        // Boolean flags: `use-notify-send = true` → `--use-notify-send`;
        // `use-notify-send = false` → emit nothing.
        if is_boolean_candidate(name) {
            if is_truthy(value_trimmed) {
                argv.push(format!("{prefix}{name}"));
                continue;
            }
            if is_falsy(value_trimmed) {
                continue;
            }
            // Fall through: not a boolean-looking value; treat as a value.
        }

        // Regular value: `--target password` style. shlex handles quoted
        // multi-value args like `keybindings = "a:b, c:d"`.
        argv.push(format!("{prefix}{name}"));
        match shlex::split(value_trimmed) {
            Some(tokens) => argv.extend(tokens),
            None => argv.push(value_trimmed.to_string()),
        }
    }

    argv
}

/// Split a config line at the first `=` or `:` — whichever comes first.
/// Falls back to the first whitespace if the line contains neither
/// (configargparse's regex allows a bare `key value` form).
fn split_key_value(line: &str) -> (&str, &str) {
    if let Some(i) = line.find(['=', ':']) {
        return (&line[..i], line[i + 1..].trim_start());
    }
    for (i, c) in line.char_indices() {
        if c.is_whitespace() {
            return (&line[..i], line[i + 1..].trim_start());
        }
    }
    (line, "")
}

/// Return `(prefix, name)` splitting a leading `--` or `-`. Keys written
/// without dashes default to the `--` prefix (long option).
fn split_dashes(key: &str) -> (&'static str, &str) {
    if let Some(rest) = key.strip_prefix("--") {
        ("--", rest)
    } else if let Some(rest) = key.strip_prefix('-') {
        ("-", rest)
    } else {
        ("--", key)
    }
}

fn is_truthy(s: &str) -> bool {
    matches!(s.to_ascii_lowercase().as_str(), "true" | "yes" | "1" | "on")
}

fn is_falsy(s: &str) -> bool {
    matches!(
        s.to_ascii_lowercase().as_str(),
        "false" | "no" | "0" | "off"
    )
}

/// Boolean (`SetTrue`) flags in the CLI. Used to disambiguate
/// `use-notify-send = true` (flag) from `--prompt true` (string value).
fn is_boolean_candidate(name: &str) -> bool {
    matches!(
        name,
        "no-help" | "no-folder" | "no-cache" | "use-notify-send"
    )
}
