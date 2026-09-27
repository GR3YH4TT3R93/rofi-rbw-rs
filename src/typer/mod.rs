pub mod dotool;
pub mod wtype;
pub mod xdotool;
pub mod ydotool;

use anyhow::{anyhow, Result};

pub const NO_TYPER: &str =
    "Could not find a valid way to type characters. \
     Please check the required dependencies.";

#[derive(Debug, Clone, Copy)]
pub enum Key {
    Enter,
    Tab,
}

pub trait Typer {
    fn name(&self) -> &'static str;
    fn supported(&self) -> bool;

    /// Returns a sentinel string for typers that can't query the active
    /// window (dotool, wtype, ydotool) and a real ID for xdotool.
    fn get_active_window(&self) -> Result<String>;

    /// Delays are in milliseconds.
    fn type_characters(
        &self,
        characters: &str,
        start_delay: u64,
        key_delay: u64,
        active_window: &str,
    ) -> Result<()>;

    fn press_key(&self, key: Key) -> Result<()>;
}

/// Fallback when no real typer is available. Every method fails with
/// the "no typer found" message, matching Python's `NoopTyper`.
pub struct NoopTyper;

impl Typer for NoopTyper {
    fn name(&self) -> &'static str { "noop" }
    fn supported(&self) -> bool { true }
    fn get_active_window(&self) -> Result<String> { Err(anyhow!(NO_TYPER)) }
    fn type_characters(&self, _: &str, _: u64, _: u64, _: &str) -> Result<()> {
        Err(anyhow!(NO_TYPER))
    }
    fn press_key(&self, _: Key) -> Result<()> { Err(anyhow!(NO_TYPER)) }
}

/// Order matches Python: `[XDoToolTyper, WTypeTyper, YDotoolTyper, DotoolTyper, NoopTyper]`.
pub fn best_typer(name: Option<&str>) -> Result<Box<dyn Typer>> {
    let available: Vec<Box<dyn Typer>> = vec![
        Box::new(xdotool::XDoToolTyper),
        Box::new(wtype::WTypeTyper),
        Box::new(ydotool::YDotoolTyper),
        Box::new(dotool::DotoolTyper),
        Box::new(NoopTyper),
    ];

    let chosen = match name {
        Some(n) => available.into_iter().find(|t| t.name() == n),
        None => available.into_iter().find(|t| t.supported()),
    };

    // Python's `next(...)` on a name-match raises StopIteration, which
    // bubbles up as RuntimeError — not caught anywhere. We surface the
    // same class of failure via NO_TYPER.
    chosen.ok_or_else(|| anyhow!(NO_TYPER))
}
