use std::process::Command;
use std::thread;
use std::time::Duration;

use anyhow::{Context, Result};

use super::{Key, Typer};
use crate::abstractionhelper::{is_installed, is_wayland};

pub struct YDotoolTyper;

impl Typer for YDotoolTyper {
    fn name(&self) -> &'static str {
        "ydotool"
    }
    fn supported(&self) -> bool {
        is_wayland() && is_installed("ydotool")
    }

    fn get_active_window(&self) -> Result<String> {
        Ok("not possible with ydotool".into())
    }

    fn type_characters(
        &self,
        characters: &str,
        start_delay: u64,
        key_delay: u64,
        _active_window: &str,
    ) -> Result<()> {
        thread::sleep(Duration::from_millis(start_delay));
        Command::new("ydotool")
            .args(["type", "--key-delay", &key_delay.to_string(), characters])
            .status()
            .context("ydotool type failed")?;
        Ok(())
    }

    fn press_key(&self, key: Key) -> Result<()> {
        let key_code = match key {
            Key::Enter => "28",
            Key::Tab => "15",
        };
        Command::new("ydotool")
            .args(["key", &format!("{key_code}:1"), &format!("{key_code}:0")])
            .status()
            .context("ydotool key failed")?;
        Ok(())
    }
}
