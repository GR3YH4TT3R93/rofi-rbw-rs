use std::process::Command;

use anyhow::Result;

use super::{Selection, Selector, extract_targets, format_targets_from_entry};
use crate::abstractionhelper::{is_installed, is_wayland, run_with_stdin};
use crate::models::{Action, DetailedEntry, DisplayFieldToken, Entry, Keybinding, Target};

pub struct Wofi;

impl Selector for Wofi {
    fn name(&self) -> &'static str {
        "wofi"
    }
    fn supported(&self) -> bool {
        is_wayland() && is_installed("wofi")
    }

    // wofi doesn't support custom keybindings from the CLI, so these are
    // never called — but the trait still requires an impl.
    fn keybinding_params(&self, _kbs: &[Keybinding]) -> Vec<String> {
        Vec::new()
    }
    fn format_keybindings_message(&self, _kbs: &[Keybinding]) -> Vec<String> {
        Vec::new()
    }

    fn show_selection(
        &self,
        entries: &[Entry],
        prompt: &str,
        _show_help: bool,
        display_fields: &[DisplayFieldToken],
        _keybindings: &[Keybinding],
        additional_args: &[String],
    ) -> Result<Selection> {
        let mut cmd = Command::new("wofi");
        cmd.args(["--dmenu", "-p", prompt]);
        cmd.args(additional_args);

        let input = self.format_entries(entries, display_fields).join("\n");
        let out = run_with_stdin(&mut cmd, &input)?;

        if out.status.success() {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let entry = self.find_entry(entries, &stdout, display_fields);
            Ok(Selection {
                targets: None,
                action: None,
                entry,
            })
        } else {
            Ok(Selection {
                targets: None,
                action: Some(Action::Cancel),
                entry: None,
            })
        }
    }

    fn select_target(
        &self,
        entry: &DetailedEntry,
        _show_help: bool,
        _keybindings: &[Keybinding],
        additional_args: &[String],
    ) -> Result<(Vec<Target>, Option<Action>)> {
        let mut cmd = Command::new("wofi");
        cmd.args(["--dmenu", "-p", "Choose target"]);
        cmd.args(additional_args);

        let input = format_targets_from_entry(entry).join("\n");
        let out = run_with_stdin(&mut cmd, &input)?;

        if out.status.code() == Some(1) {
            return Ok((Vec::new(), Some(Action::Cancel)));
        }

        let stdout = String::from_utf8_lossy(&out.stdout);
        Ok((extract_targets(&stdout), None))
    }
}
