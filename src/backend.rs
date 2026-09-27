use std::collections::HashMap;
use std::process::Command;

use anyhow::{bail, Context, Result};
use serde::Deserialize;

use crate::models::{DetailedEntry, Entry, EntryType, Field, FieldType, Target};

pub trait Backend {
    fn list_entries(&self) -> Result<Vec<Entry>>;
    fn fetch_entry(&self, entry: &Entry) -> Result<DetailedEntry>;
    fn fetch_totp(&self, entry: &Entry) -> Result<String>;
    fn sync(&self) -> Result<()>;
}

pub struct Rbw;

impl Rbw {
    pub fn new() -> Self {
        Self
    }
}

impl Default for Rbw {
    fn default() -> Self {
        Self::new()
    }
}

// ---- raw JSON shapes ------------------------------------------------------

#[derive(Deserialize)]
struct RawEntry {
    name: String,
    folder: Option<String>,
    user: Option<String>,
    #[serde(rename = "type")]
    entry_type: String,
    #[serde(default)]
    uris: Option<Vec<String>>,
}

#[derive(Deserialize)]
struct RawGet {
    folder: Option<String>,
    #[serde(default)]
    fields: Option<Vec<RawField>>,
    #[serde(default)]
    data: Option<RawData>,
    notes: Option<String>,
}

#[derive(Deserialize)]
struct RawField {
    name: Option<String>,
    value: Option<String>,
    #[serde(rename = "type")]
    field_type: Option<String>,
}

#[derive(Deserialize, Default)]
struct RawData {
    password: Option<String>,
    totp: Option<serde_json::Value>,
    uris: Option<Vec<RawUri>>,
    cardholder_name: Option<String>,
    number: Option<String>,
    brand: Option<String>,
    exp_month: Option<String>,
    exp_year: Option<String>,
    code: Option<String>,
}

#[derive(Deserialize)]
struct RawUri {
    uri: String,
}

// ---- pure parsing functions (testable) -----------------------------------

/// Parse the stdout of `rbw list --raw` into entries.
pub fn parse_list(stdout: &str) -> Result<Vec<Entry>> {
    let raw: Vec<RawEntry> = serde_json::from_str(stdout.trim())
        .context("could not parse `rbw list --raw` output as JSON")?;

    let mut entries: Vec<Entry> = raw
        .into_iter()
        .filter_map(|r| {
            Some(Entry {
                entry_type: EntryType::from_raw(&r.entry_type)?,
                name: r.name,
                folder: r.folder.unwrap_or_default(),
                username: r.user.unwrap_or_default(),
                uris: r.uris.unwrap_or_default(),
            })
        })
        .collect();

    entries.sort_by(|a, b| {
        let ka = a.folder.to_lowercase() + &a.name.to_lowercase();
        let kb = b.folder.to_lowercase() + &b.name.to_lowercase();
        ka.cmp(&kb)
    });

    Ok(entries)
}

/// Build a `DetailedEntry` from `rbw get --raw` output and the source `Entry`.
///
/// `fetch_totp` is invoked only when the raw JSON indicates a TOTP is
/// available; production wires it to `Rbw::fetch_totp`, tests pass a stub.
pub fn parse_entry<F>(entry: &Entry, raw_json: &str, fetch_totp: F) -> Result<DetailedEntry>
where
    F: Fn(&Entry) -> Result<String>,
{
    let raw: RawGet = serde_json::from_str(raw_json.trim())
        .context("could not parse `rbw get --raw` output as JSON")?;
    let data_raw = raw.data.unwrap_or_default();

    let mut data: HashMap<Target, String> = HashMap::new();
    let mut uris: Vec<String> = Vec::new();
    let mut has_totp = false;

    match entry.entry_type {
        EntryType::Login => {
            if let Some(pw) = data_raw.password {
                data.insert(Target::Password, pw);
            }
            if data_raw.totp.as_ref().is_some_and(|v| !v.is_null()) {
                has_totp = true;
                if let Ok(code) = fetch_totp(entry) {
                    if !code.is_empty() {
                        data.insert(Target::Totp, code);
                    }
                }
            }
            if let Some(raw_uris) = data_raw.uris.as_ref() {
                for (i, u) in raw_uris.iter().enumerate() {
                    uris.push(u.uri.clone());
                    data.insert(Target::Uri(i), u.uri.clone());
                }
            }
            if !entry.username.is_empty() {
                data.insert(Target::Username, entry.username.clone());
            }
            if let Some(n) = &raw.notes {
                if !n.is_empty() {
                    data.insert(Target::Notes, n.clone());
                }
            }
        }
        EntryType::Card => {
            if let Some(v) = data_raw.cardholder_name {
                data.insert(Target::Cardholder, v);
            }
            if let Some(v) = data_raw.number {
                data.insert(Target::Number, v);
            }
            if let Some(v) = data_raw.brand {
                data.insert(Target::Brand, v);
            }
            if let (Some(y), Some(m)) = (data_raw.exp_year.as_ref(), data_raw.exp_month.as_ref()) {
                data.insert(Target::Expiry, format!("{y:0>4}-{m:0>2}"));
            }
            if let Some(v) = data_raw.code {
                data.insert(Target::Code, v);
            }
            // Card notes intentionally NOT inserted — matches Python.
        }
        EntryType::Note => {
            if let Some(n) = &raw.notes {
                if !n.is_empty() {
                    data.insert(Target::Notes, n.clone());
                }
            }
        }
    }

    let notes = raw.notes;

    let fields: Vec<Field> = raw
        .fields
        .unwrap_or_default()
        .into_iter()
        .filter_map(|f| {
            let key = f.name?;
            if key.is_empty() {
                return None;
            }
            Some(Field {
                key,
                value: f.value.unwrap_or_default(),
                field_type: FieldType::from_raw(f.field_type.as_deref().unwrap_or("text")),
            })
        })
        .collect();

    Ok(DetailedEntry {
        entry_type: entry.entry_type,
        name: entry.name.clone(),
        folder: raw
            .folder
            .or_else(|| (!entry.folder.is_empty()).then(|| entry.folder.clone())),
        username: entry.username.clone(),
        fields,
        notes,
        uris,
        data,
        has_totp,
        source: entry.clone(),
    })
}

// ---- subprocess-backed Backend impl --------------------------------------

impl Backend for Rbw {
    fn list_entries(&self) -> Result<Vec<Entry>> {
        let out = Command::new("rbw")
            .args(["list", "--raw"])
            .output()
            .context("failed to spawn `rbw` — is it installed and on PATH?")?;

        if !out.status.success() {
            bail!(
                "`rbw list --raw` failed ({}): {}",
                out.status,
                String::from_utf8_lossy(&out.stderr).trim(),
            );
        }

        let stdout =
            String::from_utf8(out.stdout).context("`rbw list --raw` produced non-UTF-8 output")?;
        parse_list(&stdout)
    }

    fn fetch_entry(&self, entry: &Entry) -> Result<DetailedEntry> {
        let stdout = self.run_get_raw(entry)?;
        parse_entry(entry, &stdout, |e| self.fetch_totp(e))
    }

    fn fetch_totp(&self, entry: &Entry) -> Result<String> {
        let mut cmd = Command::new("rbw");
        cmd.args(["code", &entry.name]);
        if !entry.username.is_empty() {
            cmd.arg(&entry.username);
        }
        if !entry.folder.is_empty() {
            cmd.args(["--folder", &entry.folder]);
        }
        let out = cmd.output().context("failed to spawn `rbw code`")?;
        if !out.status.success() {
            bail!(
                "`rbw code` failed ({}): {}",
                out.status,
                String::from_utf8_lossy(&out.stderr).trim(),
            );
        }
        Ok(String::from_utf8(out.stdout)
            .context("`rbw code` produced non-UTF-8 output")?
            .trim()
            .to_string())
    }

    fn sync(&self) -> Result<()> {
        let out = Command::new("rbw")
            .arg("sync")
            .output()
            .context("failed to spawn `rbw sync`")?;
        if !out.status.success() {
            bail!(
                "`rbw sync` failed ({}): {}",
                out.status,
                String::from_utf8_lossy(&out.stderr).trim(),
            );
        }
        Ok(())
    }
}

impl Rbw {
    fn run_get_raw(&self, entry: &Entry) -> Result<String> {
        let mut cmd = Command::new("rbw");
        cmd.args(["get", "--raw", &entry.name]);
        if !entry.username.is_empty() {
            cmd.arg(&entry.username);
        }
        if !entry.folder.is_empty() {
            cmd.args(["--folder", &entry.folder]);
        }
        let out = cmd.output().context("failed to spawn `rbw get`")?;
        if !out.status.success() {
            bail!(
                "`rbw get --raw` failed ({}): {}",
                out.status,
                String::from_utf8_lossy(&out.stderr).trim(),
            );
        }
        String::from_utf8(out.stdout).context("`rbw get --raw` produced non-UTF-8 output")
    }
}
