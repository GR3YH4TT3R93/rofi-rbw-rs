use std::process::Command;
use std::thread;
use std::time::Duration;

use anyhow::{Context, Result};

use super::{Key, Typer};
use crate::abstractionhelper::{is_installed, is_wayland};

pub struct XDoToolTyper;

impl Typer for XDoToolTyper {
    fn name(&self) -> &'static str {
        "xdotool"
    }
    fn supported(&self) -> bool {
        !is_wayland() && is_installed("xdotool")
    }

    fn get_active_window(&self) -> Result<String> {
        let out = Command::new("xdotool")
            .arg("getactivewindow")
            .output()
            .context("xdotool getactivewindow failed")?;
        Ok(String::from_utf8_lossy(&out.stdout).trim_end().to_string())
    }

    fn type_characters(
        &self,
        characters: &str,
        start_delay: u64,
        key_delay: u64,
        active_window: &str,
    ) -> Result<()> {
        thread::sleep(Duration::from_millis(start_delay));

        Command::new("xdotool")
            .args([
                "windowactivate",
                "--sync",
                active_window,
                "type",
                "--clearmodifiers",
                "--delay",
                &key_delay.to_string(),
                "--",
                characters,
            ])
            .status()
            .context("xdotool type failed")?;

        // Workaround for https://github.com/jordansissel/xdotool/issues/43
        Command::new("xdotool")
            .args(["keyup", "Shift_L", "Shift_R", "Alt_L", "Alt_R"])
            .status()
            .context("xdotool keyup failed")?;

        Ok(())
    }

    fn press_key(&self, key: Key) -> Result<()> {
        let key_name = match key {
            Key::Enter => "Return",
            Key::Tab => "Tab",
        };
        Command::new("xdotool")
            .args(["key", key_name])
            .status()
            .context("xdotool key failed")?;
        Ok(())
    }
}
