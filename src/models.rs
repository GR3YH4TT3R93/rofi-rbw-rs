use std::collections::HashMap;
use std::hash::{Hash, Hasher};

// =========================================================================
// Target
// =========================================================================

/// Rust-side equivalent of Python's normalized `Target` string.
///
/// Known values are matched to variants for exhaustive `match`; anything
/// else falls through to `Custom(String)` and is looked up as a custom
/// field by name (`field.key == target.raw().trim_end_matches(" (field)")`).
#[derive(Debug, Clone)]
pub enum Target {
    Username,
    Password,
    Totp,
    Number,
    Cardholder,
    Brand,
    Expiry,
    Code,
    Notes,
    Menu,
    Uri(usize), // 0-based; `Uri(0)` ↔ "uri", `Uri(1)` ↔ "uri 2"
    Delay,
    Enter,
    Tab,
    Custom(String),
}

// Equality and hashing delegate to `raw()`, matching Python's `__eq__`.
impl PartialEq for Target {
    fn eq(&self, other: &Self) -> bool {
        self.raw() == other.raw()
    }
}
impl Eq for Target {}
impl Hash for Target {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.raw().hash(state);
    }
}

impl Target {
    pub fn raw(&self) -> String {
        match self {
            Self::Username => "username".into(),
            Self::Password => "password".into(),
            Self::Totp => "totp".into(),
            Self::Number => "number".into(),
            Self::Cardholder => "cardholder".into(),
            Self::Brand => "brand".into(),
            Self::Expiry => "expiry".into(),
            Self::Code => "code".into(),
            Self::Notes => "notes".into(),
            Self::Menu => "menu".into(),
            Self::Uri(0) => "uri".into(),
            Self::Uri(n) => format!("uri {}", n + 1),
            Self::Delay => "delay".into(),
            Self::Enter => "enter".into(),
            Self::Tab => "tab".into(),
            Self::Custom(s) => s.clone(),
        }
    }

    /// Mirrors Python's `Target(input)` normalization:
    ///  * known names (any case) → lowercased known variant
    ///  * "uri" / "uri N" → `Uri(0)` / `Uri(N-1)`
    ///  * anything else → kept verbatim as `Custom`
    pub fn parse(input: &str) -> Option<Self> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return None;
        }
        let lower = trimmed.to_ascii_lowercase();

        match lower.as_str() {
            "username" => return Some(Self::Username),
            "password" => return Some(Self::Password),
            "totp" => return Some(Self::Totp),
            "number" => return Some(Self::Number),
            "cardholder" => return Some(Self::Cardholder),
            "brand" => return Some(Self::Brand),
            "expiry" => return Some(Self::Expiry),
            "code" => return Some(Self::Code),
            "notes" => return Some(Self::Notes),
            "menu" => return Some(Self::Menu),
            "delay" => return Some(Self::Delay),
            "enter" => return Some(Self::Enter),
            "tab" => return Some(Self::Tab),
            "uri" => return Some(Self::Uri(0)),
            _ => {}
        }

        if let Some(rest) = lower.strip_prefix("uri ") {
            if let Ok(n) = rest.parse::<usize>() {
                if n >= 1 {
                    return Some(Self::Uri(n - 1));
                }
            }
        }

        Some(Self::Custom(trimmed.to_string()))
    }

    /// Name as it appears on the left of a target-menu line, e.g. "Password".
    /// Reverse of `format_further_item_name`.
    pub fn from_display(s: &str) -> Option<Self> {
        // Strip the " (field)" suffix for parsing, then re-attach it as
        // part of the raw so the custom-field lookup hits the right key.
        Self::parse(s)
    }

    pub fn is_uri(&self) -> bool {
        matches!(self, Self::Uri(_))
    }

    pub fn uri_index(&self) -> Option<usize> {
        if let Self::Uri(n) = self {
            Some(*n)
        } else {
            None
        }
    }
}

// =========================================================================
// Action
// =========================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Type,
    Copy,
    Print,
    Sync,
    Cancel,
}

impl Action {
    pub fn value(self) -> &'static str {
        match self {
            Self::Type => "type",
            Self::Copy => "copy",
            Self::Print => "print",
            Self::Sync => "sync",
            Self::Cancel => "cancel",
        }
    }
}

// =========================================================================
// EntryType + Entry
// =========================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryType {
    Login,
    Card,
    Note,
}

impl EntryType {
    pub fn from_raw(s: &str) -> Option<Self> {
        match s {
            "login" => Some(Self::Login),
            "card" => Some(Self::Card),
            "note" => Some(Self::Note),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Entry {
    pub name: String,
    pub folder: String,
    pub username: String,
    pub entry_type: EntryType,
    pub uris: Vec<String>,
}

impl Entry {
    /// sha1(name || folder || username), hex-encoded. Used as cache key.
    pub fn hashed(&self) -> String {
        use sha1::{Digest, Sha1};
        let mut h = Sha1::new();
        h.update(self.name.as_bytes());
        if !self.folder.is_empty() {
            h.update(self.folder.as_bytes());
        }
        if !self.username.is_empty() {
            h.update(self.username.as_bytes());
        }
        let digest = h.finalize();
        digest.iter().map(|b| format!("{b:02x}")).collect()
    }
}

// =========================================================================
// Field
// =========================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldType {
    Hidden,
    Text,
    Boolean,
    Linked,
}

impl FieldType {
    pub fn from_raw(s: &str) -> Self {
        match s {
            "hidden" => Self::Hidden,
            "boolean" => Self::Boolean,
            "linked" => Self::Linked,
            _ => Self::Text,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Field {
    pub key: String,
    pub value: String,
    pub field_type: FieldType,
}

impl Field {
    /// Hidden fields mask: first char + `*` × (len - 1). Empty string stays empty.
    pub fn masked_string(&self) -> String {
        if self.field_type == FieldType::Hidden && !self.value.is_empty() {
            let mut chars = self.value.chars();
            let first = chars.next().unwrap();
            format!("{first}{}", "*".repeat(chars.count()))
        } else {
            self.value.clone()
        }
    }
}

// =========================================================================
// Keybinding
// =========================================================================

#[derive(Debug, Clone)]
pub struct Keybinding {
    pub shortcut: String,
    pub action: Option<Action>,
    pub targets: Vec<Target>,
}

// =========================================================================
// DisplayFieldToken
// =========================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayFieldToken {
    NameOnly,
    NameWithFolder,
    Folder,
    User,
    FirstUri,
}

impl DisplayFieldToken {
    pub fn value(self) -> &'static str {
        match self {
            Self::NameOnly => "name_only",
            Self::NameWithFolder => "name_with_folder",
            Self::Folder => "folder",
            Self::User => "user",
            Self::FirstUri => "uri", // NOT "first_uri"
        }
    }

    pub fn name(self) -> &'static str {
        // Uppercase form used as the regex named-capture group.
        match self {
            Self::NameOnly => "name_only",
            Self::NameWithFolder => "name_with_folder",
            Self::Folder => "folder",
            Self::User => "user",
            Self::FirstUri => "uri",
        }
    }

    pub fn from_raw(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "name_only" => Some(Self::NameOnly),
            "name_with_folder" => Some(Self::NameWithFolder),
            "folder" => Some(Self::Folder),
            "user" => Some(Self::User),
            "uri" => Some(Self::FirstUri),
            _ => None,
        }
    }
}

// =========================================================================
// DetailedEntry
// =========================================================================

#[derive(Debug, Clone)]
pub struct DetailedEntry {
    pub entry_type: EntryType,
    pub name: String,
    pub folder: Option<String>,
    pub username: String,
    pub fields: Vec<Field>,
    pub notes: Option<String>,
    pub uris: Vec<String>, // NEW — populated for Login, empty otherwise
    pub data: HashMap<Target, String>,
    pub has_totp: bool,
    pub source: Entry,
}

impl DetailedEntry {
    pub fn get(&self, target: &Target) -> Option<String> {
        // 1. Precomputed map (password, totp, uris, card fields, etc.)
        if let Some(v) = self.data.get(target) {
            return Some(v.clone());
        }

        // 2. Field lookup by key, stripping " (field)" if present.
        let raw = target.raw();
        let key = raw.strip_suffix(" (field)").unwrap_or(&raw);
        self.fields
            .iter()
            .find(|f| f.key == key)
            .map(|f| f.value.clone())
    }

    pub fn default_target(&self) -> Vec<Target> {
        match self.entry_type {
            EntryType::Login => vec![Target::Username, Target::Password],
            EntryType::Card => vec![Target::Number, Target::Expiry, Target::Code],
            EntryType::Note => vec![Target::Notes],
        }
    }

    pub fn default_autotype_target(&self) -> Vec<Target> {
        match self.entry_type {
            EntryType::Login => vec![Target::Username, Target::Tab, Target::Password],
            EntryType::Card => vec![
                Target::Number,
                Target::Tab,
                Target::Expiry,
                Target::Tab,
                Target::Code,
            ],
            EntryType::Note => vec![Target::Notes],
        }
    }

    /// Read the `_autotype` custom field, if present, into a target sequence.
    /// Python splits on `:` and parses each token as a `TypeTarget`.
    pub fn autotype_sequence(&self) -> Option<Vec<Target>> {
        let field = self.fields.iter().find(|f| f.key == "_autotype")?;
        let seq: Vec<Target> = field
            .value
            .trim()
            .split(':')
            .filter_map(Target::parse)
            .collect();
        if seq.is_empty() {
            None
        } else {
            Some(seq)
        }
    }
}
