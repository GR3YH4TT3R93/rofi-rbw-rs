mod common;
use common::*;

use rofi_rbw_rs::models::{DetailedEntry, EntryType, FieldType, Target};

// ---------------------------------------------------------------------------
// Target::parse
// ---------------------------------------------------------------------------

#[test]
fn parse_known_names_any_case() {
    assert_eq!(Target::parse("password").unwrap().raw(), "password");
    assert_eq!(Target::parse("PASSWORD").unwrap().raw(), "password");
    assert_eq!(Target::parse("PassWord").unwrap().raw(), "password");
    assert_eq!(Target::parse("username").unwrap().raw(), "username");
    assert_eq!(Target::parse("totp").unwrap().raw(), "totp");
    assert_eq!(Target::parse("notes").unwrap().raw(), "notes");
    assert_eq!(Target::parse("menu").unwrap().raw(), "menu");
    assert_eq!(Target::parse("delay").unwrap().raw(), "delay");
    assert_eq!(Target::parse("enter").unwrap().raw(), "enter");
    assert_eq!(Target::parse("tab").unwrap().raw(), "tab");
}

#[test]
fn parse_card_targets() {
    assert_eq!(Target::parse("number").unwrap().raw(), "number");
    assert_eq!(Target::parse("cardholder").unwrap().raw(), "cardholder");
    assert_eq!(Target::parse("brand").unwrap().raw(), "brand");
    assert_eq!(Target::parse("expiry").unwrap().raw(), "expiry");
    assert_eq!(Target::parse("code").unwrap().raw(), "code");
}

#[test]
fn parse_uri_variants() {
    assert_eq!(Target::parse("uri").unwrap(), Target::Uri(0));
    assert_eq!(Target::parse("URI").unwrap(), Target::Uri(0));
    assert_eq!(Target::parse("uri 1").unwrap(), Target::Uri(0));
    assert_eq!(Target::parse("uri 2").unwrap(), Target::Uri(1));
    assert_eq!(Target::parse("URI 3").unwrap(), Target::Uri(2));
}

#[test]
fn target_uri_raw_roundtrip() {
    assert_eq!(Target::Uri(0).raw(), "uri");
    assert_eq!(Target::Uri(1).raw(), "uri 2");
    assert_eq!(Target::Uri(9).raw(), "uri 10");
}

#[test]
fn parse_custom_field_preserves_case() {
    let t = Target::parse("My Custom Field").unwrap();
    assert_eq!(t.raw(), "My Custom Field");
    let t = Target::parse("CustomCased").unwrap();
    assert_eq!(t.raw(), "CustomCased");
}

#[test]
fn parse_empty_or_whitespace_is_none() {
    assert!(Target::parse("").is_none());
    assert!(Target::parse("   ").is_none());
}

#[test]
fn parse_trims_whitespace() {
    assert_eq!(Target::parse("  password  ").unwrap(), Target::Password);
    assert_eq!(Target::parse("\tusername\n").unwrap(), Target::Username);
}

#[test]
fn target_eq_by_raw() {
    assert_eq!(Target::Uri(0), Target::Uri(0));
    assert_ne!(Target::Uri(0), Target::Uri(1));
    assert_eq!(Target::Password, Target::parse("password").unwrap());
    assert_eq!(Target::Password, Target::parse("PASSWORD").unwrap());
}

#[test]
fn target_hash_by_raw() {
    use std::collections::HashSet;
    let mut set = HashSet::new();
    set.insert(Target::Password);
    set.insert(Target::parse("password").unwrap());
    set.insert(Target::Username);
    assert_eq!(set.len(), 2);
}

// ---------------------------------------------------------------------------
// Field::masked_string
// ---------------------------------------------------------------------------

#[test]
fn masked_string_hidden() {
    assert_eq!(
        field("x", "hunter2", FieldType::Hidden).masked_string(),
        "h******"
    );
}

#[test]
fn masked_string_single_char() {
    assert_eq!(field("x", "a", FieldType::Hidden).masked_string(), "a");
}

#[test]
fn masked_string_empty_is_empty() {
    assert_eq!(field("x", "", FieldType::Hidden).masked_string(), "");
}

#[test]
fn masked_string_text_passthrough() {
    assert_eq!(
        field("x", "visible", FieldType::Text).masked_string(),
        "visible"
    );
}

#[test]
fn masked_string_unicode_counts_chars_not_bytes() {
    assert_eq!(
        field("x", "héllo", FieldType::Hidden).masked_string(),
        "h****"
    );
}

// ---------------------------------------------------------------------------
// Entry::hashed
// ---------------------------------------------------------------------------

#[test]
fn entry_hash_stable_and_hex40() {
    let a = entry("name", "folder", "user");
    let b = entry("name", "folder", "user");
    assert_eq!(a.hashed(), b.hashed());
    assert_eq!(a.hashed().len(), 40);
}

#[test]
fn entry_hash_changes_on_any_component() {
    let base = entry("n", "f", "u");
    assert_ne!(base.hashed(), entry("N", "f", "u").hashed());
    assert_ne!(base.hashed(), entry("n", "F", "u").hashed());
    assert_ne!(base.hashed(), entry("n", "f", "U").hashed());
}

#[test]
fn entry_hash_skips_empty_folder_and_user() {
    let a = entry("n", "", "");
    let b = entry("n", "", "");
    assert_eq!(a.hashed(), b.hashed());
    assert_ne!(a.hashed(), entry("m", "", "").hashed());
}

// ---------------------------------------------------------------------------
// DetailedEntry defaults
// ---------------------------------------------------------------------------

#[test]
fn default_targets_login() {
    let e = detailed(EntryType::Login);
    assert_eq!(e.default_target(), vec![Target::Username, Target::Password]);
    assert_eq!(
        e.default_autotype_target(),
        vec![Target::Username, Target::Tab, Target::Password]
    );
}

#[test]
fn default_targets_card() {
    let e = detailed(EntryType::Card);
    assert_eq!(
        e.default_target(),
        vec![Target::Number, Target::Expiry, Target::Code]
    );
    assert_eq!(
        e.default_autotype_target(),
        vec![
            Target::Number,
            Target::Tab,
            Target::Expiry,
            Target::Tab,
            Target::Code,
        ]
    );
}

#[test]
fn default_targets_note() {
    let e = detailed(EntryType::Note);
    assert_eq!(e.default_target(), vec![Target::Notes]);
    assert_eq!(e.default_autotype_target(), vec![Target::Notes]);
}

// ---------------------------------------------------------------------------
// DetailedEntry::autotype_sequence
// ---------------------------------------------------------------------------

#[test]
fn autotype_sequence_none_without_field() {
    let e = detailed(EntryType::Login);
    assert!(e.autotype_sequence().is_none());
}

#[test]
fn autotype_sequence_parses_colon_separated() {
    let mut e = detailed(EntryType::Login);
    e.fields
        .push(field("_autotype", "username:tab:password", FieldType::Text));
    assert_eq!(
        e.autotype_sequence(),
        Some(vec![Target::Username, Target::Tab, Target::Password])
    );
}

#[test]
fn autotype_sequence_trims_each_token() {
    let mut e = detailed(EntryType::Login);
    e.fields.push(field(
        "_autotype",
        " username : tab : password ",
        FieldType::Text,
    ));
    assert_eq!(
        e.autotype_sequence(),
        Some(vec![Target::Username, Target::Tab, Target::Password])
    );
}

// ---------------------------------------------------------------------------
// DetailedEntry::get
// ---------------------------------------------------------------------------

#[test]
fn get_reads_from_data_map() {
    let mut e = detailed(EntryType::Login);
    e.data.insert(Target::Password, "hunter2".into());
    assert_eq!(e.get(&Target::Password).as_deref(), Some("hunter2"));
}

#[test]
fn get_falls_back_to_field_by_key() {
    let mut e = detailed(EntryType::Login);
    e.fields
        .push(field("Custom", "customvalue", FieldType::Text));
    let t = Target::parse("Custom").unwrap();
    assert_eq!(e.get(&t).as_deref(), Some("customvalue"));
}

#[test]
fn get_strips_field_suffix() {
    let mut e = detailed(EntryType::Login);
    e.fields
        .push(field("username", "custom-override", FieldType::Text));
    let t = Target::parse("username (field)").unwrap();
    assert_eq!(e.get(&t).as_deref(), Some("custom-override"));
}

#[test]
fn get_returns_none_for_missing() {
    let e: DetailedEntry = detailed(EntryType::Login);
    assert!(e.get(&Target::Password).is_none());
}
