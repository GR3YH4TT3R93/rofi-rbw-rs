use std::time::Duration;

use anyhow::Result;

use crate::args::{parse_arguments, Args};
use crate::backend::{Backend, Rbw};
use crate::cache::Cache;
use crate::clipboarder::{best_clipboarder, Clipboarder};
use crate::models::{Action, DetailedEntry, Target};
use crate::notifier::{DesktopNotifier, Notifier};
use crate::selector::{best_selector, Selector};
use crate::typer::{best_typer, Key, Typer};

enum Flow {
    Done,
    Restart,
}

pub struct RofiRbw {
    args: Args,
    rbw: Box<dyn Backend>,
    selector: Box<dyn Selector>,
    typer: Box<dyn Typer>,
    clipboarder: Box<dyn Clipboarder>,
    notifier: Box<dyn Notifier>,
    active_window: String,
}

impl RofiRbw {
    pub fn new() -> Result<Self> {
        let args = parse_arguments()?;
        let rbw: Box<dyn Backend> = Box::new(Rbw::new());
        let selector = best_selector(args.selector.as_deref())?;
        let typer = best_typer(args.typer.as_deref())?;
        let clipboarder = best_clipboarder(args.clipboarder.as_deref())?;
        let notifier: Box<dyn Notifier> = Box::new(DesktopNotifier);
        let active_window = typer.get_active_window()?;

        Ok(Self {
            args,
            rbw,
            selector,
            typer,
            clipboarder,
            notifier,
            active_window,
        })
    }

    pub fn run(&mut self) -> Result<()> {
        loop {
            match self.run_iteration()? {
                Flow::Done => return Ok(()),
                Flow::Restart => continue,
            }
        }
    }

    fn run_iteration(&mut self) -> Result<Flow> {
        let mut cache = Cache::new()?;
        let mut entries = self.rbw.list_entries()?;

        if self.args.use_cache {
            entries = cache.sorted(entries);
        }
        let mut sel = self.selector.show_selection(
            &entries,
            &self.args.prompt,
            self.args.show_help,
            &self.args.display_fields,
            &self.args.parsed_keybindings,
            &self.args.selector_args,
        )?;

        if sel.action == Some(Action::Sync) {
            self.rbw.sync()?;
            let entries = self.rbw.list_entries()?;
            sel = self.selector.show_selection(
                &entries,
                &self.args.prompt,
                self.args.show_help,
                &self.args.display_fields,
                &self.args.parsed_keybindings,
                &self.args.selector_args,
            )?;
        }

        if sel.action == Some(Action::Cancel) {
            return Ok(Flow::Done);
        }

        let Some(selected_entry) = sel.entry else {
            return Ok(Flow::Done);
        };

        let entry = self.rbw.fetch_entry(&selected_entry)?;

        if self.args.use_cache {
            cache.update(&selected_entry)?;
        }

        if sel.targets.is_some() {
            self.args.targets = sel.targets;
        }
        if let Some(action) = sel.action {
            self.args.action = action;
        }

        if self
            .args
            .targets
            .as_ref()
            .is_some_and(|t| t.contains(&Target::Menu))
        {
            match self.show_target_menu(&entry)? {
                Flow::Restart => return Ok(Flow::Restart),
                Flow::Done => {}
            }
        }

        self.execute_action(&entry)?;
        Ok(Flow::Done)
    }

    fn show_target_menu(&mut self, entry: &DetailedEntry) -> Result<Flow> {
        let (targets, action) = self.selector.select_target(
            entry,
            self.args.show_help,
            &self.args.parsed_menu_keybindings,
            &self.args.selector_args,
        )?;

        if action == Some(Action::Cancel) {
            return Ok(Flow::Restart);
        }

        self.args.targets = Some(targets);
        if let Some(a) = action {
            self.args.action = a;
        }
        Ok(Flow::Done)
    }

    fn execute_action(&self, entry: &DetailedEntry) -> Result<()> {
        let targets = self.configure_targets(entry);
        match self.args.action {
            Action::Type => self.type_targets(entry, &targets),
            Action::Copy => self.copy_targets(entry, &targets),
            Action::Print => {
                let lines: Vec<String> = targets.iter().filter_map(|t| entry.get(t)).collect();
                println!("{}", lines.join("\n"));
                Ok(())
            }
            Action::Sync | Action::Cancel => Ok(()),
        }
    }

    fn configure_targets(&self, entry: &DetailedEntry) -> Vec<Target> {
        if let Some(targets) = &self.args.targets {
            if !targets.is_empty() {
                return targets.clone();
            }
        }
        if self.args.action == Action::Type {
            return entry
                .autotype_sequence()
                .unwrap_or_else(|| entry.default_autotype_target());
        }
        entry.default_target()
    }

    fn type_targets(&self, entry: &DetailedEntry, targets: &[Target]) -> Result<()> {
        for target in targets {
            match target {
                Target::Delay => {
                    std::thread::sleep(Duration::from_millis(self.args.action_sequence_delay));
                }
                Target::Enter => self.typer.press_key(Key::Enter)?,
                Target::Tab => self.typer.press_key(Key::Tab)?,
                _ => {
                    let value = entry.get(target).unwrap_or_default();
                    self.typer.type_characters(
                        &value,
                        self.args.start_delay,
                        self.args.key_delay,
                        &self.active_window,
                    )?;
                }
            }
        }

        if targets.contains(&Target::Password) && entry.has_totp {
            if let Some(totp) = entry.get(&Target::Totp) {
                if !totp.is_empty() {
                    self.clipboarder.copy_to_clipboard(&totp)?;
                    if self.args.use_notify_send {
                        self.notifier.notify("totp copied to clipboard")?;
                    }
                }
            }
        }
        Ok(())
    }

    fn copy_targets(&self, entry: &DetailedEntry, targets: &[Target]) -> Result<()> {
        for target in targets {
            if *target == Target::Delay {
                std::thread::sleep(Duration::from_millis(self.args.action_sequence_delay));
                continue;
            }
            let value = entry.get(target).unwrap_or_default();
            if value.is_empty() {
                continue;
            }
            self.clipboarder.copy_to_clipboard(&value)?;
            if self.args.use_notify_send {
                self.notifier
                    .notify(&format!("{} copied to clipboard", target.raw()))?;
            }
        }

        if targets.len() == 1 && targets[0] == Target::Password {
            self.clipboarder.clear_clipboard_after(self.args.clear)?;
        }
        Ok(())
    }
}
