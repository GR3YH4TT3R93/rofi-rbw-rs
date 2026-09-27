mod common;
use anyhow::{anyhow, Result};
use common::{entry, entry_of_type};

use rofi_rbw_rs::backend::{parse_entry, parse_list};
use rofi_rbw_rs::models::{Entry, EntryType, FieldType, Target};

/// Stand-in for `Rbw::fetch_totp` so tests don't shell out to `rbw code`.
fn fake_totp(_: &rofi_rbw_rs::models::Entry) -> Result<String> {
    Ok("123456".to_string())
}

fn no_totp(_: &rofi_rbw_rs::models::Entry) -> Result<String> {
    Err(anyhow!("no totp"))
}

// ===========================================================================
// parse_list
// ===========================================================================

#[test]
fn list_parses_login_card_note() {
    let json = r#"[
        {"name":"github","folder":"work","user":"alice","type":"login","uris":["https://github.com"]},
        {"name":"visa","folder":"","user":"","type":"card","uris":[]},
        {"name":"wifi","folder":"home","user":"","type":"note","uris":[]}
    ]"#;
    let entries = parse_list(json).unwrap();
    assert_eq!(entries.len(), 3);

    let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
    assert!(names.contains(&"github"));
    assert!(names.contains(&"visa"));
    assert!(names.contains(&"wifi"));
}

#[test]
fn list_filters_out_unknown_types() {
    let json = r#"[
        {"name":"ok","folder":"","user":"","type":"login","uris":[]},
        {"name":"ssh-key","folder":"","user":"","type":"ssh","uris":[]},
        {"name":"identity","folder":"","user":"","type":"identity","uris":[]}
    ]"#;
    let entries = parse_list(json).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].name, "ok");
}

#[test]
fn list_handles_null_folder_and_user() {
    let json = r#"[{"name":"x","folder":null,"user":null,"type":"login","uris":null}]"#;
    let entries = parse_list(json).unwrap();
    assert_eq!(entries[0].folder, "");
    assert_eq!(entries[0].username, "");
    assert!(entries[0].uris.is_empty());
}

#[test]
fn list_handles_missing_uris_field() {
    let json = r#"[{"name":"x","folder":"","user":"","type":"login"}]"#;
    let entries = parse_list(json).unwrap();
    assert!(entries[0].uris.is_empty());
}

#[test]
fn list_sorts_by_folder_then_name_case_insensitive() {
    // Python: `x.folder.lower() + x.name.lower()`
    let json = r#"[
        {"name":"Zebra","folder":"alpha","user":"","type":"login","uris":[]},
        {"name":"apple","folder":"Beta","user":"","type":"login","uris":[]},
        {"name":"apricot","folder":"alpha","user":"","type":"login","uris":[]},
        {"name":"banana","folder":"beta","user":"","type":"login","uris":[]}
    ]"#;
    let entries = parse_list(json).unwrap();
    let order: Vec<(&str, &str)> = entries
        .iter()
        .map(|e| (e.folder.as_str(), e.name.as_str()))
        .collect();
    assert_eq!(
        order,
        vec![
            ("alpha", "apricot"),
            ("alpha", "Zebra"),
            ("Beta", "apple"),
            ("beta", "banana"),
        ]
    );
}

#[test]
fn list_rejects_malformed_json() {
    assert!(parse_list("not json").is_err());
    assert!(parse_list("{").is_err());
}

#[test]
fn list_empty_array() {
    assert!(parse_list("[]").unwrap().is_empty());
}

// ===========================================================================
// parse_entry — login
// ===========================================================================

#[test]
fn login_parses_password_and_username() {
    let e = entry("github", "work", "alice");
    let json = r#"{
        "folder": "work",
        "fields": [],
        "data": {"password": "hunter2"},
        "notes": null
    }"#;
    let d = parse_entry(&e, json, no_totp).unwrap();
    assert_eq!(d.get(&Target::Password).as_deref(), Some("hunter2"));
    assert_eq!(d.get(&Target::Username).as_deref(), Some("alice"));
}

#[test]
fn login_populates_uris_from_get_not_from_list() {
    // Critical: URIs come from `rbw get --raw` data.uris, not from
    // the entry's list-time uris. This was a bug caught by clippy earlier.
    let mut e = entry("github", "work", "alice");
    e.uris = vec!["stale-list-uri".into()];
    let json = r#"{
        "folder": "work",
        "fields": [],
        "data": {
            "password": "x",
            "uris": [{"uri": "https://a.com"}, {"uri": "https://b.com"}]
        },
        "notes": null
    }"#;
    let d = parse_entry(&e, json, no_totp).unwrap();
    assert_eq!(d.uris, vec!["https://a.com", "https://b.com"]);
    assert_eq!(d.get(&Target::Uri(0)).as_deref(), Some("https://a.com"));
    assert_eq!(d.get(&Target::Uri(1)).as_deref(), Some("https://b.com"));
    assert!(d.get(&Target::Uri(2)).is_none());
}

#[test]
fn login_notes_go_into_data_map() {
    let e = entry("x", "", "");
    let json = r#"{
        "folder": "",
        "fields": [],
        "data": {"password": "p"},
        "notes": "some notes"
    }"#;
    let d = parse_entry(&e, json, no_totp).unwrap();
    assert_eq!(d.get(&Target::Notes).as_deref(), Some("some notes"));
}

#[test]
fn login_empty_notes_not_inserted() {
    let e = entry("x", "", "");
    let json = r#"{
        "folder": "",
        "fields": [],
        "data": {"password": "p"},
        "notes": ""
    }"#;
    let d = parse_entry(&e, json, no_totp).unwrap();
    assert!(d.get(&Target::Notes).is_none());
}

#[test]
fn login_totp_present_when_json_has_it_and_fetch_succeeds() {
    let e = entry("x", "", "");
    let json = r#"{
        "folder": "",
        "fields": [],
        "data": {"password": "p", "totp": true},
        "notes": null
    }"#;
    let d = parse_entry(&e, json, fake_totp).unwrap();
    assert!(d.has_totp);
    assert_eq!(d.get(&Target::Totp).as_deref(), Some("123456"));
}

#[test]
fn login_totp_flag_true_but_fetch_fails() {
    let e = entry("x", "", "");
    let json = r#"{
        "folder": "",
        "fields": [],
        "data": {"password": "p", "totp": true},
        "notes": null
    }"#;
    let d = parse_entry(&e, json, no_totp).unwrap();
    // has_totp is true — the JSON says so. But no code was stored.
    assert!(d.has_totp);
    assert!(d.get(&Target::Totp).is_none());
}

#[test]
fn login_totp_null_leaves_flag_false() {
    let e = entry("x", "", "");
    let json = r#"{
        "folder": "",
        "fields": [],
        "data": {"password": "p", "totp": null},
        "notes": null
    }"#;
    let d = parse_entry(&e, json, fake_totp).unwrap();
    assert!(!d.has_totp);
    assert!(d.get(&Target::Totp).is_none());
}

// ===========================================================================
// parse_entry — card
// ===========================================================================

#[test]
fn card_composes_expiry_from_month_year() {
    let e = entry_of_type("visa", "", "", EntryType::Card);
    let json = r#"{
        "folder": "",
        "fields": [],
        "data": {
            "cardholder_name": "Alice Doe",
            "number": "4111111111111111",
            "brand": "Visa",
            "exp_month": "4",
            "exp_year": "2027",
            "code": "123"
        },
        "notes": null
    }"#;
    let d = parse_entry(&e, json, no_totp).unwrap();
    assert_eq!(d.get(&Target::Cardholder).as_deref(), Some("Alice Doe"));
    assert_eq!(d.get(&Target::Number).as_deref(), Some("4111111111111111"));
    assert_eq!(d.get(&Target::Brand).as_deref(), Some("Visa"));
    assert_eq!(d.get(&Target::Expiry).as_deref(), Some("2027-04"));
    assert_eq!(d.get(&Target::Code).as_deref(), Some("123"));
}

#[test]
fn card_zero_pads_month_and_year() {
    let e = entry_of_type("visa", "", "", EntryType::Card);
    let json = r#"{
        "folder": "",
        "fields": [],
        "data": {"exp_month": "3", "exp_year": "27"},
        "notes": null
    }"#;
    let d = parse_entry(&e, json, no_totp).unwrap();
    assert_eq!(d.get(&Target::Expiry).as_deref(), Some("0027-03"));
}

#[test]
fn card_notes_not_inserted() {
    // Faithful to Python: Card.__getitem__(NOTES) returns None even when
    // card.notes exists.
    let e = entry_of_type("visa", "", "", EntryType::Card);
    let json = r#"{
        "folder": "",
        "fields": [],
        "data": {"number": "4"},
        "notes": "secret note"
    }"#;
    let d = parse_entry(&e, json, no_totp).unwrap();
    assert!(d.get(&Target::Notes).is_none());
}

// ===========================================================================
// parse_entry — note
// ===========================================================================

#[test]
fn note_puts_notes_in_data() {
    let e = Entry {
        name: "wifi".into(),
        folder: "home".into(),
        username: "".into(),
        entry_type: EntryType::Note,
        uris: vec![],
    };
    let json = r#"{
        "folder": "home",
        "fields": [],
        "data": {},
        "notes": "ssid: mynet\npassword: secret"
    }"#;
    let d = parse_entry(&e, json, no_totp).unwrap();
    assert_eq!(
        d.get(&Target::Notes).as_deref(),
        Some("ssid: mynet\npassword: secret")
    );
}

// ===========================================================================
// parse_entry — fields
// ===========================================================================

#[test]
fn fields_are_parsed_with_types() {
    let e = entry("x", "", "");
    let json = r#"{
        "folder": "",
        "fields": [
            {"name": "text_field", "value": "visible", "type": "text"},
            {"name": "hidden_field", "value": "secret", "type": "hidden"},
            {"name": "bool_field", "value": "true", "type": "boolean"},
            {"name": "linked_field", "value": "https://x", "type": "linked"}
        ],
        "data": {"password": "p"},
        "notes": null
    }"#;
    let d = parse_entry(&e, json, no_totp).unwrap();
    assert_eq!(d.fields.len(), 4);

    let text = d.fields.iter().find(|f| f.key == "text_field").unwrap();
    assert_eq!(text.field_type, FieldType::Text);
    assert_eq!(text.masked_string(), "visible");

    let hidden = d.fields.iter().find(|f| f.key == "hidden_field").unwrap();
    assert_eq!(hidden.field_type, FieldType::Hidden);
    assert_eq!(hidden.masked_string(), "s*****");

    let boolean = d.fields.iter().find(|f| f.key == "bool_field").unwrap();
    assert_eq!(boolean.field_type, FieldType::Boolean);

    let linked = d.fields.iter().find(|f| f.key == "linked_field").unwrap();
    assert_eq!(linked.field_type, FieldType::Linked);
}

#[test]
fn fields_skip_empty_names() {
    let e = entry("x", "", "");
    let json = r#"{
        "folder": "",
        "fields": [
            {"name": "", "value": "x", "type": "text"},
            {"name": null, "value": "y", "type": "text"},
            {"name": "good", "value": "z", "type": "text"}
        ],
        "data": {"password": "p"},
        "notes": null
    }"#;
    let d = parse_entry(&e, json, no_totp).unwrap();
    assert_eq!(d.fields.len(), 1);
    assert_eq!(d.fields[0].key, "good");
}

#[test]
fn fields_default_to_text_type_when_missing() {
    let e = entry("x", "", "");
    let json = r#"{
        "folder": "",
        "fields": [{"name": "k", "value": "v"}],
        "data": {"password": "p"},
        "notes": null
    }"#;
    let d = parse_entry(&e, json, no_totp).unwrap();
    assert_eq!(d.fields[0].field_type, FieldType::Text);
}

// ===========================================================================
// parse_entry — folder resolution
// ===========================================================================

#[test]
fn folder_from_raw_wins() {
    let e = entry("x", "from-entry", "");
    let json = r#"{"folder": "from-raw", "fields": [], "data": {"password": "p"}, "notes": null}"#;
    let d = parse_entry(&e, json, no_totp).unwrap();
    assert_eq!(d.folder.as_deref(), Some("from-raw"));
}

#[test]
fn folder_falls_back_to_entry_when_raw_is_null() {
    let e = entry("x", "from-entry", "");
    let json = r#"{"folder": null, "fields": [], "data": {"password": "p"}, "notes": null}"#;
    let d = parse_entry(&e, json, no_totp).unwrap();
    assert_eq!(d.folder.as_deref(), Some("from-entry"));
}

#[test]
fn folder_none_when_both_empty() {
    let e = entry("x", "", "");
    let json = r#"{"folder": null, "fields": [], "data": {"password": "p"}, "notes": null}"#;
    let d = parse_entry(&e, json, no_totp).unwrap();
    assert!(d.folder.is_none());
}
