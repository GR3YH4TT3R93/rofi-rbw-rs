use std::process::Command;
use std::thread;
use std::time::Duration;

use anyhow::{Context, Result};

use super::{Key, Typer};
use crate::abstractionhelper::{is_installed, run_with_stdin};

pub struct DotoolTyper;

impl Typer for DotoolTyper {
    fn name(&self) -> &'static str {
        "dotool"
    }
    fn supported(&self) -> bool {
        is_installed("dotool")
    }

    fn get_active_window(&self) -> Result<String> {
        Ok("not possible with dotool".into())
    }

    fn type_characters(
        &self,
        characters: &str,
        start_delay: u64,
        key_delay: u64,
        _active_window: &str,
    ) -> Result<()> {
        thread::sleep(Duration::from_millis(start_delay));
        let input = format!("typedelay {key_delay}\ntype {characters}");
        let mut cmd = Command::new("dotool");
        run_with_stdin(&mut cmd, &input).context("dotool failed")?;
        Ok(())
    }

    fn press_key(&self, key: Key) -> Result<()> {
        let key_name = match key {
            Key::Enter => "enter",
            Key::Tab => "tab",
        };
        let mut cmd = Command::new("dotool");
        run_with_stdin(&mut cmd, &format!("key {key_name}")).context("dotool failed")?;
        Ok(())
    }
}
