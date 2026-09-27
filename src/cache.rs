use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};

use crate::models::Entry;
use crate::paths::cache_file;

pub struct Cache {
    path: PathBuf,
    /// hashed → score. Values are `f64` because `update()` adds 1.1 per hit.
    cache: HashMap<String, f64>,
}

impl Cache {
    /// Uses the standard cache file location from `$XDG_CACHE_HOME` / `~/.cache`.
    pub fn new() -> Result<Self> {
        Self::at(cache_file())
    }

    /// Uses an explicit file path. Intended for tests; also useful if a
    /// future `--cache-file` flag is added.
    pub fn at(path: PathBuf) -> Result<Self> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)
                    .with_context(|| format!("failed to create cache dir {}", parent.display()))?;
            }
        }
        Ok(Self {
            path,
            cache: HashMap::new(),
        })
    }

    pub fn sorted(&mut self, entries: Vec<Entry>) -> Vec<Entry> {
        let mut by_hash: HashMap<String, Entry> =
            entries.into_iter().map(|e| (e.hashed(), e)).collect();

        let mut sorted = Vec::new();
        if let Ok(text) = fs::read_to_string(&self.path) {
            for line in text.lines() {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                let Some((amount_str, hashed)) = line.split_once(' ') else {
                    continue;
                };
                if let Some(entry) = by_hash.remove(hashed) {
                    sorted.push(entry);
                    if let Ok(n) = amount_str.parse::<i64>() {
                        self.cache.insert(hashed.to_string(), n as f64);
                    }
                }
            }
        }

        sorted.extend(by_hash.into_values());
        sorted
    }

    pub fn update(&mut self, entry: &Entry) -> Result<()> {
        let hashed = entry.hashed();
        let current = self.cache.get(&hashed).copied().unwrap_or(0.0);
        self.cache.insert(hashed, current + 1.1);
        self.write()
    }

    fn write(&self) -> Result<()> {
        let mut items: Vec<(&String, &f64)> = self.cache.iter().collect();
        items.sort_by(|a, b| b.1.partial_cmp(a.1).unwrap_or(std::cmp::Ordering::Equal));

        let mut out = String::new();
        for (hashed, n) in items {
            let floored = n.max(0.0).floor() as i64;
            out.push_str(&format!("{floored} {hashed}\n"));
        }

        fs::write(&self.path, out)
            .with_context(|| format!("failed to write cache file {}", self.path.display()))?;
        Ok(())
    }
}
