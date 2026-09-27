pub mod bemenu;
pub mod fuzzel;
pub mod rofi;
pub mod wofi;

use anyhow::{anyhow, Result};

use crate::models::{
    Action, DetailedEntry, DisplayFieldToken, Entry, EntryType, Keybinding, Target,
};

pub const NO_SELECTOR: &str = "Could not find a valid way to show the selection. \
     Please check the required dependencies.";

#[derive(Debug, Clone)]
pub struct Selection {
    pub targets: Option<Vec<Target>>,
    pub action: Option<Action>,
    pub entry: Option<Entry>,
}

pub trait Selector {
    fn name(&self) -> &'static str;
    fn supported(&self) -> bool;

    fn format_field(&self, entry: &Entry, token: DisplayFieldToken) -> String {
        default_format_field(entry, token)
    }

    fn keybinding_params(&self, kbs: &[Keybinding]) -> Vec<String>;

    fn format_keybindings_message(&self, kbs: &[Keybinding]) -> Vec<String>;

    fn format_entries(&self, entries: &[Entry], tokens: &[DisplayFieldToken]) -> Vec<String> {
        if entries.is_empty() || tokens.is_empty() {
            return Vec::new();
        }
        let cells: Vec<Vec<String>> = entries
            .iter()
            .map(|e| tokens.iter().map(|t| self.format_field(e, *t)).collect())
            .collect();

        let mut widths = vec![0usize; tokens.len()];
        for row in &cells {
            for (i, c) in row.iter().enumerate() {
                widths[i] = widths[i].max(c.chars().count());
            }
        }

        cells
            .into_iter()
            .map(|row| {
                row.into_iter()
                    .enumerate()
                    .map(|(i, c)| format!("{:<width$}", c, width = widths[i]))
                    .collect::<Vec<_>>()
                    .join("  ")
            })
            .collect()
    }

    fn find_entry(
        &self,
        entries: &[Entry],
        formatted: &str,
        display_fields: &[DisplayFieldToken],
    ) -> Option<Entry> {
        if display_fields.is_empty() {
            return None;
        }

        let group_names: Vec<String> = display_fields
            .iter()
            .map(|t| t.name().to_string())
            .collect();

        let pattern = format!(
            "^{}$",
            group_names
                .iter()
                .map(|n| format!("(?P<{n}>.*?)"))
                .collect::<Vec<_>>()
                .join(" {2,}")
        );
        let re = regex::Regex::new(&pattern).ok()?;
        let line = formatted.trim();
        let caps = re.captures(line)?;

        'outer: for entry in entries {
            for (i, token) in display_fields.iter().enumerate() {
                let want = self.format_field(entry, *token);
                let got = caps
                    .name(&group_names[i])
                    .map(|m| m.as_str().trim())
                    .unwrap_or("");
                if got != want {
                    continue 'outer;
                }
            }
            return Some(entry.clone());
        }
        None
    }

    fn show_selection(
        &self,
        entries: &[Entry],
        prompt: &str,
        show_help: bool,
        display_fields: &[DisplayFieldToken],
        keybindings: &[Keybinding],
        additional_args: &[String],
    ) -> Result<Selection>;

    fn select_target(
        &self,
        entry: &DetailedEntry,
        show_help: bool,
        keybindings: &[Keybinding],
        additional_args: &[String],
    ) -> Result<(Vec<Target>, Option<Action>)>;
}

// ---- free formatting functions shared by all selectors --------------------

pub fn default_format_field(entry: &Entry, token: DisplayFieldToken) -> String {
    match token {
        DisplayFieldToken::NameOnly => entry.name.clone(),
        DisplayFieldToken::NameWithFolder => {
            if entry.folder.is_empty() {
                entry.name.clone()
            } else {
                format!("{}/{}", entry.folder, entry.name)
            }
        }
        DisplayFieldToken::Folder => entry.folder.clone(),
        DisplayFieldToken::User => entry.username.clone(),
        DisplayFieldToken::FirstUri => entry.uris.first().cloned().unwrap_or_default(),
    }
}

pub fn format_action_and_targets(kb: &Keybinding) -> String {
    if !kb.targets.is_empty() && kb.targets.contains(&Target::Menu) {
        "Menu".into()
    } else if kb.action == Some(Action::Sync) {
        "Sync logins".into()
    } else if !kb.targets.is_empty() {
        let act = kb.action.map(Action::value).unwrap_or("");
        format!(
            "{} {}",
            capitalize(act),
            kb.targets
                .iter()
                .map(|t| t.raw())
                .collect::<Vec<_>>()
                .join(", ")
        )
    } else {
        capitalize(kb.action.map(Action::value).unwrap_or(""))
    }
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

pub fn best_selector(name: Option<&str>) -> Result<Box<dyn Selector>> {
    let available: Vec<Box<dyn Selector>> = vec![
        Box::new(rofi::Rofi),
        Box::new(wofi::Wofi),
        Box::new(fuzzel::Fuzzel),
        Box::new(bemenu::Bemenu),
    ];

    let chosen = match name {
        Some(n) => available.into_iter().find(|s| s.name() == n),
        None => available.into_iter().find(|s| s.supported()),
    };

    chosen.ok_or_else(|| anyhow!(NO_SELECTOR))
}

// ---- helpers from the Python base class ----------------------------------

pub fn extract_targets(output: &str) -> Vec<Target> {
    output
        .trim()
        .lines()
        .filter_map(|line| {
            let head = line.split(':').next().unwrap_or("").trim();
            Target::parse(head)
        })
        .collect()
}

pub fn format_targets_from_entry(entry: &DetailedEntry) -> Vec<String> {
    match entry.entry_type {
        EntryType::Login => format_targets_from_login(entry),
        EntryType::Card => format_targets_from_card(entry),
        EntryType::Note => format_targets_from_note(entry),
    }
}

fn format_targets_from_login(entry: &DetailedEntry) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(v) = entry.get(&Target::Username).filter(|s| !s.is_empty()) {
        out.push(format!("Username: {v}"));
    }
    if let Some(pw) = entry.get(&Target::Password).filter(|s| !s.is_empty()) {
        let first = pw.chars().next().unwrap();
        out.push(format!(
            "Password: {first}{}",
            "*".repeat(pw.chars().count() - 1)
        ));
    }
    if let Some(totp) = entry.get(&Target::Totp).filter(|s| !s.is_empty()) {
        out.push(format!("TOTP: {totp}"));
    }
    if let Some(n) = entry.get(&Target::Notes).filter(|s| !s.is_empty()) {
        out.push(format!("Notes: {n}"));
    }
    match entry.uris.len() {
        0 => {}
        1 => out.push(format!("URI: {}", entry.source.uris[0])),
        _ => {
            for (i, uri) in entry.source.uris.iter().enumerate() {
                out.push(format!("URI {}: {}", i + 1, uri));
            }
        }
    }
    for field in &entry.fields {
        out.push(format!(
            "{}: {}",
            format_further_item_name(&field.key),
            field.masked_string()
        ));
    }
    out
}

fn format_targets_from_card(entry: &DetailedEntry) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(v) = entry.get(&Target::Number).filter(|s| !s.is_empty()) {
        out.push(format!("Number: {v}"));
    }
    if let Some(v) = entry.get(&Target::Cardholder).filter(|s| !s.is_empty()) {
        out.push(format!("Cardholder: {v}"));
    }
    if let Some(v) = entry.get(&Target::Brand).filter(|s| !s.is_empty()) {
        out.push(format!("Brand: {v}"));
    }
    if let Some(v) = entry.get(&Target::Expiry).filter(|s| !s.is_empty()) {
        out.push(format!("Expiry: {v}"));
    }
    if let Some(c) = entry.get(&Target::Code).filter(|s| !s.is_empty()) {
        let first = c.chars().next().unwrap();
        out.push(format!(
            "Code: {first}{}",
            "*".repeat(c.chars().count() - 1)
        ));
    }
    if let Some(n) = entry.get(&Target::Notes).filter(|s| !s.is_empty()) {
        out.push(format!("Notes: {n}"));
    }
    for field in &entry.fields {
        out.push(format!(
            "{}: {}",
            format_further_item_name(&field.key),
            field.masked_string()
        ));
    }
    out
}

fn format_targets_from_note(entry: &DetailedEntry) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(n) = entry.get(&Target::Notes).filter(|s| !s.is_empty()) {
        out.push(format!("Notes: {}", n.replace('\n', "|")));
    }
    for field in &entry.fields {
        out.push(format!(
            "{}: {}",
            format_further_item_name(&field.key),
            field.masked_string()
        ));
    }
    out
}

fn format_further_item_name(key: &str) -> String {
    use regex::Regex;
    use std::sync::OnceLock;
    static URI_RE: OnceLock<Regex> = OnceLock::new();
    let re = URI_RE.get_or_init(|| Regex::new(r"^URI \d+$").unwrap());

    let lower = key.to_ascii_lowercase();
    if matches!(lower.as_str(), "username" | "password" | "totp") || re.is_match(key) {
        format!("{key} (field)")
    } else {
        key.to_string()
    }
}
