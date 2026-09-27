use std::process::Command;

use anyhow::{Context, Result};

pub trait Notifier {
    fn notify(&self, message: &str) -> Result<()>;
}

pub struct DesktopNotifier;

impl Notifier for DesktopNotifier {
    fn notify(&self, message: &str) -> Result<()> {
        Command::new("notify-send")
            .args(["-u", "normal", "rofi-rbw", message])
            .status()
            .context("failed to spawn `notify-send`")?;
        Ok(())
    }
}
