//! Shared test fixtures. Imported by each test file via `mod common;`.

#![allow(dead_code)]

use std::path::PathBuf;

use tempfile::TempDir;

use rofi_rbw_rs::cache::Cache;
use rofi_rbw_rs::models::{DetailedEntry, Entry, EntryType, Field, FieldType};

pub fn entry(name: &str, folder: &str, username: &str) -> Entry {
    Entry {
        name: name.into(),
        folder: folder.into(),
        username: username.into(),
        entry_type: EntryType::Login,
        uris: Vec::new(),
    }
}

pub fn entry_of_type(name: &str, folder: &str, username: &str, entry_type: EntryType) -> Entry {
    Entry {
        name: name.into(),
        folder: folder.into(),
        username: username.into(),
        entry_type,
        uris: Vec::new(),
    }
}

pub fn detailed(entry_type: EntryType) -> DetailedEntry {
    DetailedEntry {
        entry_type,
        name: "name".into(),
        folder: None,
        username: String::new(),
        fields: Vec::new(),
        notes: None,
        uris: Vec::new(),
        data: Default::default(),
        has_totp: false,
        source: entry("name", "", ""),
    }
}

pub fn field(key: &str, value: &str, field_type: FieldType) -> Field {
    Field {
        key: key.into(),
        value: value.into(),
        field_type,
    }
}

pub fn temp_cache() -> (Cache, TempDir) {
    let dir = TempDir::new().expect("create tempdir");
    let path: PathBuf = dir.path().join("rofi-rbw.runcache");
    let cache = Cache::at(path).expect("create cache");
    (cache, dir)
}
