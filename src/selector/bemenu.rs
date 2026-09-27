use std::process::Command;

use anyhow::Result;

use super::{Selection, Selector, extract_targets, format_targets_from_entry};
use crate::abstractionhelper::{is_installed, run_with_stdin};
use crate::models::{Action, DetailedEntry, DisplayFieldToken, Entry, Keybinding, Target};

pub struct Bemenu;

impl Selector for Bemenu {
    fn name(&self) -> &'static str {
        "bemenu"
    }
    fn supported(&self) -> bool {
        is_installed("bemenu")
    }

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
        let mut cmd = Command::new("bemenu");
        cmd.args(["-p", prompt]);
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
        let mut cmd = Command::new("bemenu");
        cmd.args(["-p", "Choose target"]);
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
