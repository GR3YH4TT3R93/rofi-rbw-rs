use std::path::PathBuf;

/// `$XDG_CACHE_HOME/rofi-rbw.runcache` or `$HOME/.cache/rofi-rbw.runcache`.
pub fn cache_file() -> PathBuf {
    if let Some(xdg) = std::env::var_os("XDG_CACHE_HOME") {
        PathBuf::from(xdg).join("rofi-rbw.runcache")
    } else {
        home().join(".cache").join("rofi-rbw.runcache")
    }
}

/// Candidate locations for the config file, in precedence order (first wins).
///
/// Mirrors Python's `paths.py` for the flat `rofi-rbw.rc` form, and extends
/// it with the directory-style `rofi-rbw/config.rc` form (XDG-friendly).
///
/// Precedence (highest → lowest):
///   1. $XDG_CONFIG_HOME/rofi-rbw.rc             (or ~/.config/rofi-rbw.rc)
///   2. $XDG_CONFIG_HOME/rofi-rbw/config.rc      (or ~/.config/rofi-rbw/config.rc)
///   3. $XDG_CONFIG_DIRS/*/rofi-rbw.rc           (default /etc/xdg/rofi-rbw.rc)
///   4. $XDG_CONFIG_DIRS/*/rofi-rbw/config.rc    (default /etc/xdg/rofi-rbw/config.rc)
///
/// The flat form keeps existing users working; the directory form is
/// available to anyone who prefers the modern convention.
pub fn config_file_locations() -> Vec<PathBuf> {
    let config_home = if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME") {
        PathBuf::from(xdg)
    } else {
        home().join(".config")
    };

    let config_dirs: Vec<PathBuf> = if let Some(dirs) = std::env::var_os("XDG_CONFIG_DIRS") {
        std::env::split_paths(&dirs).collect()
    } else {
        vec![PathBuf::from("/etc/xdg")]
    };

    let mut locations = Vec::new();

    // User: flat first (matches Python), then directory form.
    locations.push(config_home.join("rofi-rbw.rc"));
    locations.push(config_home.join("rofi-rbw").join("config.rc"));

    // System: same order per directory.
    for dir in config_dirs {
        locations.push(dir.join("rofi-rbw.rc"));
        locations.push(dir.join("rofi-rbw").join("config.rc"));
    }

    locations
}

fn home() -> PathBuf {
    // `dirs::home_dir` would be more robust (handles `HOME` unset on some
    // platforms, uses passwd), but Python's `Path.home()` on Linux is
    // effectively `$HOME` — so we match.
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/"))
}
