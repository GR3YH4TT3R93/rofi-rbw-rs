use std::cell::RefCell;
use std::process::Command;
use std::thread;
use std::time::Duration;

use anyhow::{Context, Result};

use super::Clipboarder;
use crate::abstractionhelper::{is_installed, is_wayland, run_with_stdin};

pub struct XSelClipboarder {
    last_copied: RefCell<Option<String>>,
}

impl XSelClipboarder {
    pub fn new() -> Self {
        Self {
            last_copied: RefCell::new(None),
        }
    }

    fn fetch_clipboard_content(&self) -> Result<String> {
        let out = Command::new("xsel")
            .args(["--output", "--clipboard"])
            .output()
            .context("xsel --output failed")?;
        Ok(String::from_utf8_lossy(&out.stdout).into_owned())
    }
}

impl Clipboarder for XSelClipboarder {
    fn name(&self) -> &'static str {
        "xsel"
    }
    fn supported(&self) -> bool {
        !is_wayland() && is_installed("xsel")
    }

    fn copy_to_clipboard(&self, characters: &str) -> Result<()> {
        let mut cmd = Command::new("xsel");
        cmd.args(["--input", "--clipboard"]);
        run_with_stdin(&mut cmd, characters).context("xsel failed")?;
        *self.last_copied.borrow_mut() = Some(characters.to_string());
        Ok(())
    }

    fn clear_clipboard_after(&self, clear: u64) -> Result<()> {
        if clear == 0 {
            return Ok(());
        }
        thread::sleep(Duration::from_secs(clear));

        let last = self.last_copied.borrow().clone();
        if let Some(last) = last {
            if self.fetch_clipboard_content()? == last {
                Command::new("xsel")
                    .args(["--clear", "--clipboard"])
                    .status()
                    .context("xsel --clear failed")?;
                *self.last_copied.borrow_mut() = None;
            }
        }
        Ok(())
    }
}
