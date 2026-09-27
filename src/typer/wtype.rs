use std::process::Command;
use std::thread;
use std::time::Duration;

use anyhow::{Context, Result};

use super::{Key, Typer};
use crate::abstractionhelper::{is_installed, is_wayland, run_with_stdin};

pub struct WTypeTyper;

impl Typer for WTypeTyper {
    fn name(&self) -> &'static str {
        "wtype"
    }
    fn supported(&self) -> bool {
        is_wayland() && is_installed("wtype")
    }

    fn get_active_window(&self) -> Result<String> {
        Ok("not possible with wtype".into())
    }

    fn type_characters(
        &self,
        characters: &str,
        start_delay: u64,
        key_delay: u64,
        _active_window: &str,
    ) -> Result<()> {
        thread::sleep(Duration::from_millis(start_delay));
        let mut cmd = Command::new("wtype");
        if key_delay > 0 {
            cmd.args(["-d", &key_delay.to_string()]);
        }
        cmd.args(["--", characters]);
        cmd.status().context("wtype failed")?;
        Ok(())
    }

    fn press_key(&self, key: Key) -> Result<()> {
        let key_name = match key {
            Key::Enter => "return",
            Key::Tab => "tab",
        };
        Command::new("wtype")
            .args(["-k", key_name])
            .status()
            .context("wtype failed")?;
        Ok(())
    }
}

// silence dead import if unused
#[allow(unused_imports)]
use run_with_stdin as _;
