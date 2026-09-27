pub mod wlclip;
pub mod xclip;
pub mod xsel;

use anyhow::{anyhow, Result};

pub const NO_CLIPBOARDER: &str =
    "Could not find a valid way to copy to clipboard. \
     Please check the required dependencies.";

pub trait Clipboarder {
    fn name(&self) -> &'static str;
    fn supported(&self) -> bool;

    fn copy_to_clipboard(&self, characters: &str) -> Result<()>;

    /// Seconds. Backends treat `0` as "do nothing". The call blocks for
    /// `clear` seconds (mirrors Python's `time.sleep(clear)`).
    fn clear_clipboard_after(&self, clear: u64) -> Result<()>;
}

/// Fallback used when nothing else is installed. Every method errors,
/// matching Python's `NoopClipboarder`.
pub struct NoopClipboarder;

impl Clipboarder for NoopClipboarder {
    fn name(&self) -> &'static str { "noop" }
    fn supported(&self) -> bool { true }
    fn copy_to_clipboard(&self, _: &str) -> Result<()> { Err(anyhow!(NO_CLIPBOARDER)) }
    fn clear_clipboard_after(&self, _: u64) -> Result<()> { Err(anyhow!(NO_CLIPBOARDER)) }
}

/// Order matches Python: `[XSel, XClip, WlClipboarder, Noop]`.
pub fn best_clipboarder(name: Option<&str>) -> Result<Box<dyn Clipboarder>> {
    let available: Vec<Box<dyn Clipboarder>> = vec![
        Box::new(xsel::XSelClipboarder::new()),
        Box::new(xclip::XClipClipboarder::new()),
        Box::new(wlclip::WlClipboarder::new()),
        Box::new(NoopClipboarder),
    ];

    let chosen = match name {
        Some(n) => available.into_iter().find(|c| c.name() == n),
        None => available.into_iter().find(|c| c.supported()),
    };

    chosen.ok_or_else(|| anyhow!(NO_CLIPBOARDER))
}
