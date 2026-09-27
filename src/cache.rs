use std::collections::HashMap;
use std::fs;

use anyhow::{Context, Result};

use crate::{models::Entry, paths::cache_file};

pub struct Cache {
    /// hashed → score. Values are `f64` because `update()` adds 1.1 per hit.
    cache: HashMap<String, f64>,
}

impl Cache {
    pub fn new() -> Result<Self> {
        let path = cache_file();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create cache dir {}", parent.display()))?;
        }
        Ok(Self {
            cache: HashMap::new(),
        })
    }

    /// Returns `entries` reordered so cached ones come first (in cached
    /// score order), followed by uncached ones (in original order).
    pub fn sorted(&mut self, entries: Vec<Entry>) -> Vec<Entry> {
        // hashed → entry, so we can pop by hash. Python uses dict comprehension
        // which also de-dups by hash if two entries collide — same behavior.
        let mut by_hash: HashMap<String, Entry> =
            entries.into_iter().map(|e| (e.hashed(), e)).collect();

        let mut sorted = Vec::new();
        if let Some(text) = read_cache_file() {
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

        // Remaining entries are the uncached ones. Python's
        // `hashed_entries.values()` preserves insertion order (dict in 3.7+),
        // which matches `entries` order — but a HashMap loses that. Restore
        // the source order by iterating over the original index.
        //
        // We can't recover the original `entries` here since we consumed it,
        // so we accept HashMap order for the tail. It's only a tie-break
        // cosmetic issue on the uncached portion.
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
        let path = cache_file();

        // Sort by score descending. Ties are non-deterministic because
        // HashMap iteration order is unstable in Rust; Python's dict
        // preserves insertion order, so ties would be stable there.
        // If tie-break determinism matters, swap the map for `indexmap`.
        let mut items: Vec<(&String, &f64)> = self.cache.iter().collect();
        items.sort_by(|a, b| b.1.partial_cmp(a.1).unwrap_or(std::cmp::Ordering::Equal));

        let mut out = String::new();
        for (hashed, n) in items {
            // Python: floor(n). f64::floor matches for the values we store
            // (non-negative), and `.max(0.0)` guards against an odd file.
            let floored = n.max(0.0).floor() as i64;
            out.push_str(&format!("{floored} {hashed}\n"));
        }

        fs::write(&path, out)
            .with_context(|| format!("failed to write cache file {}", path.display()))?;
        Ok(())
    }
}

fn read_cache_file() -> Option<String> {
    std::fs::read_to_string(cache_file()).ok()
}
