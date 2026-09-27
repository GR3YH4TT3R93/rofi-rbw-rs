mod common;
use common::*;

use rofi_rbw_rs::models::{
    Action, DetailedEntry, DisplayFieldToken, Entry, EntryType, Keybinding, Target,
};
use rofi_rbw_rs::selector::{
    default_format_field, extract_targets, format_action_and_targets, format_targets_from_entry,
    Selection, Selector,
};

// ---------------------------------------------------------------------------
// extract_targets
// ---------------------------------------------------------------------------

#[test]
fn extract_targets_basic() {
    let out = "Password: hunter2\nUsername: alice\nTOTP: 123456\n";
    assert_eq!(
        extract_targets(out),
        vec![Target::Password, Target::Username, Target::Totp]
    );
}

#[test]
fn extract_targets_uri_indices() {
    let out = "URI: https://a.com\nURI 2: https://b.com\n";
    assert_eq!(extract_targets(out), vec![Target::Uri(0), Target::Uri(1)]);
}

#[test]
fn extract_targets_custom_field() {
    let out = "MyField: value\n";
    let result = extract_targets(out);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].raw(), "MyField");
}

#[test]
fn extract_targets_empty_input() {
    assert!(extract_targets("").is_empty());
    assert!(extract_targets("   \n  ").is_empty());
}

// ---------------------------------------------------------------------------
// format_action_and_targets
// ---------------------------------------------------------------------------

#[test]
fn format_kb_menu() {
    let kb = Keybinding {
        shortcut: "Alt+m".into(),
        action: None,
        targets: vec![Target::Menu],
    };
    assert_eq!(format_action_and_targets(&kb), "Menu");
}

#[test]
fn format_kb_sync() {
    let kb = Keybinding {
        shortcut: "Alt+s".into(),
        action: Some(Action::Sync),
        targets: vec![],
    };
    assert_eq!(format_action_and_targets(&kb), "Sync logins");
}

#[test]
fn format_kb_action_with_targets() {
    let kb = Keybinding {
        shortcut: "Alt+1".into(),
        action: Some(Action::Type),
        targets: vec![Target::Username, Target::Password],
    };
    assert_eq!(format_action_and_targets(&kb), "Type username, password");
}

#[test]
fn format_kb_action_only() {
    let kb = Keybinding {
        shortcut: "Alt+t".into(),
        action: Some(Action::Copy),
        targets: vec![],
    };
    assert_eq!(format_action_and_targets(&kb), "Copy");
}

// ---------------------------------------------------------------------------
// default_format_field
// ---------------------------------------------------------------------------

#[test]
fn format_field_all_tokens() {
    let e = Entry {
        name: "GitHub".into(),
        folder: "work".into(),
        username: "alice".into(),
        entry_type: EntryType::Login,
        uris: vec!["https://github.com".into()],
    };
    assert_eq!(
        default_format_field(&e, DisplayFieldToken::NameOnly),
        "GitHub"
    );
    assert_eq!(
        default_format_field(&e, DisplayFieldToken::NameWithFolder),
        "work/GitHub"
    );
    assert_eq!(default_format_field(&e, DisplayFieldToken::Folder), "work");
    assert_eq!(default_format_field(&e, DisplayFieldToken::User), "alice");
    assert_eq!(
        default_format_field(&e, DisplayFieldToken::FirstUri),
        "https://github.com"
    );
}

#[test]
fn format_field_no_folder() {
    let e = entry("GitHub", "", "alice");
    assert_eq!(
        default_format_field(&e, DisplayFieldToken::NameWithFolder),
        "GitHub"
    );
    assert_eq!(default_format_field(&e, DisplayFieldToken::FirstUri), "");
}

// ---------------------------------------------------------------------------
// format_targets_from_entry
// ---------------------------------------------------------------------------

#[test]
fn login_menu_masks_password() {
    let mut e = detailed(EntryType::Login);
    e.data.insert(Target::Username, "alice".into());
    e.data.insert(Target::Password, "hunter2".into());
    let out = format_targets_from_entry(&e);
    assert!(out.contains(&"Username: alice".to_string()));
    assert!(out.contains(&"Password: h******".to_string()));
    assert!(!out.iter().any(|l| l.contains("hunter2")));
}

#[test]
fn note_menu_replaces_newlines() {
    let mut e = detailed(EntryType::Note);
    e.data.insert(Target::Notes, "line1\nline2".into());
    assert_eq!(format_targets_from_entry(&e), vec!["Notes: line1|line2"]);
}

#[test]
fn card_menu_shows_composite_expiry() {
    let mut e = detailed(EntryType::Card);
    e.data.insert(Target::Number, "4111111111111111".into());
    e.data.insert(Target::Expiry, "2027-04".into());
    e.data.insert(Target::Code, "123".into());
    let out = format_targets_from_entry(&e);
    assert!(out.contains(&"Number: 4111111111111111".to_string()));
    assert!(out.contains(&"Expiry: 2027-04".to_string()));
    assert!(out.contains(&"Code: 1**".to_string()));
}

// ---------------------------------------------------------------------------
// Selector provided methods (format_entries, find_entry, format_field)
// ---------------------------------------------------------------------------

struct DummySelector;

impl Selector for DummySelector {
    fn name(&self) -> &'static str {
        "dummy"
    }
    fn supported(&self) -> bool {
        true
    }
    fn keybinding_params(&self, _: &[Keybinding]) -> Vec<String> {
        Vec::new()
    }
    fn format_keybindings_message(&self, _: &[Keybinding]) -> Vec<String> {
        Vec::new()
    }
    fn show_selection(
        &self,
        _: &[Entry],
        _: &str,
        _: bool,
        _: &[DisplayFieldToken],
        _: &[Keybinding],
        _: &[String],
    ) -> anyhow::Result<Selection> {
        unreachable!("DummySelector: show_selection not implemented")
    }
    fn select_target(
        &self,
        _: &DetailedEntry,
        _: bool,
        _: &[Keybinding],
        _: &[String],
    ) -> anyhow::Result<(Vec<Target>, Option<Action>)> {
        unreachable!("DummySelector: select_target not implemented")
    }
}

#[test]
fn format_entries_pads_columns() {
    let s = DummySelector;
    let entries = vec![entry("a", "f", "alice"), entry("longer", "f", "bob")];
    let fields = vec![DisplayFieldToken::NameOnly, DisplayFieldToken::User];
    let out = s.format_entries(&entries, &fields);
    assert_eq!(out.len(), 2);
    assert_eq!(out[0].len(), out[1].len(), "columns must align");
    assert!(out[0].starts_with("a     "));
}

#[test]
fn format_entries_empty() {
    let s = DummySelector;
    assert!(s
        .format_entries(&[], &[DisplayFieldToken::NameOnly])
        .is_empty());
    assert!(s.format_entries(&[entry("x", "", "")], &[]).is_empty());
}

#[test]
fn find_entry_single_column() {
    let s = DummySelector;
    let entries = vec![entry("alpha", "f", "alice"), entry("beta", "f", "bob")];
    let fields = vec![DisplayFieldToken::NameOnly];
    let found = s.find_entry(&entries, "beta", &fields).unwrap();
    assert_eq!(found.name, "beta");
}

#[test]
fn find_entry_multi_column_roundtrip() {
    let s = DummySelector;
    let entries = vec![entry("alpha", "f", "alice"), entry("beta", "g", "bob")];
    let fields = vec![DisplayFieldToken::NameWithFolder, DisplayFieldToken::User];
    let formatted = s.format_entries(&entries, &fields);
    for (line, expected) in formatted.iter().zip(entries.iter()) {
        let found = s.find_entry(&entries, line, &fields).unwrap();
        assert_eq!(found.name, expected.name);
    }
}

#[test]
fn find_entry_returns_none_for_garbage() {
    let s = DummySelector;
    let entries = vec![entry("alpha", "f", "alice")];
    let fields = vec![DisplayFieldToken::NameOnly];
    assert!(s.find_entry(&entries, "not an entry", &fields).is_none());
}
