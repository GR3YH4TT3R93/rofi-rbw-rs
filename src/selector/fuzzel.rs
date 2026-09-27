use std::process::Command;

use anyhow::{anyhow, Result};

use super::{
    extract_targets, format_action_and_targets, format_targets_from_entry, Selection, Selector,
};
use crate::abstractionhelper::{is_installed, is_wayland, run_with_stdin};
use crate::models::{Action, DetailedEntry, DisplayFieldToken, Entry, Keybinding, Target};

pub struct Fuzzel;

impl Selector for Fuzzel {
    fn name(&self) -> &'static str {
        "fuzzel"
    }
    fn supported(&self) -> bool {
        is_wayland() && is_installed("fuzzel")
    }

    fn keybinding_params(&self, kbs: &[Keybinding]) -> Vec<String> {
        kbs.iter()
            .enumerate()
            .map(|(i, kb)| {
                format!(
                    "--override=key-bindings.custom-{}={}",
                    i + 1,
                    translate_shortcut(&kb.shortcut)
                )
            })
            .collect()
    }

    fn format_keybindings_message(&self, kbs: &[Keybinding]) -> Vec<String> {
        let msg = kbs
            .iter()
            .map(|kb| format!("{}: {}", kb.shortcut, format_action_and_targets(kb)))
            .collect::<Vec<_>>()
            .join(" | ");
        vec!["--mesg".into(), msg]
    }

    fn show_selection(
        &self,
        entries: &[Entry],
        prompt: &str,
        show_help: bool,
        display_fields: &[DisplayFieldToken],
        keybindings: &[Keybinding],
        additional_args: &[String],
    ) -> Result<Selection> {
        let mut cmd = Command::new("fuzzel");
        cmd.args(["--dmenu", "--index", "-p", prompt]);
        cmd.args(self.keybinding_params(keybindings));
        cmd.args(additional_args);
        if show_help && !keybindings.is_empty() {
            cmd.args(self.format_keybindings_message(keybindings));
        }

        let input = self.format_entries(entries, display_fields).join("\n");
        let out = run_with_stdin(&mut cmd, &input)?;
        let code = out.status.code().unwrap_or(-1);

        if code == 1 {
            return Ok(Selection {
                targets: None,
                action: Some(Action::Cancel),
                entry: None,
            });
        }

        let (action, targets) = if code >= 10 {
            let kb = keybindings
                .get((code - 10) as usize)
                .ok_or_else(|| anyhow!("fuzzel returned unknown custom keybinding code {code}"))?;
            (kb.action, Some(kb.targets.clone()))
        } else {
            (None, None)
        };
        let stdout = String::from_utf8_lossy(&out.stdout);
        let entry = stdout
            .trim()
            .parse::<usize>()
            .ok()
            .and_then(|i| entries.get(i).cloned());

        Ok(Selection {
            targets,
            action,
            entry,
        })
    }

    fn select_target(
        &self,
        entry: &DetailedEntry,
        show_help: bool,
        keybindings: &[Keybinding],
        additional_args: &[String],
    ) -> Result<(Vec<Target>, Option<Action>)> {
        let mut cmd = Command::new("fuzzel");
        cmd.args(["--dmenu", "-p", "Choose target"]);
        cmd.args(self.keybinding_params(keybindings));
        cmd.args(additional_args);
        if show_help && !keybindings.is_empty() {
            cmd.args(self.format_keybindings_message(keybindings));
        }

        let input = format_targets_from_entry(entry).join("\n");
        let out = run_with_stdin(&mut cmd, &input)?;
        let code = out.status.code().unwrap_or(-1);

        if code == 1 {
            return Ok((Vec::new(), Some(Action::Cancel)));
        }

        let action = if code >= 10 {
            keybindings
                .get((code - 10) as usize)
                .and_then(|kb| kb.action)
        } else {
            None
        };

        let stdout = String::from_utf8_lossy(&out.stdout);
        Ok((extract_targets(&stdout), action))
    }
}

fn translate_shortcut(shortcut: &str) -> String {
    shortcut
        .split('+')
        .map(|tok| if tok == "Alt" { "Mod1" } else { tok })
        .collect::<Vec<_>>()
        .join("+")
}
