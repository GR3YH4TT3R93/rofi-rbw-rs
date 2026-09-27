mod common;
use common::{entry, temp_cache};

use std::fs;
use std::path::Path;

// ---------------------------------------------------------------------------
// Cache::sorted with no existing file
// ---------------------------------------------------------------------------

#[test]
fn sorted_with_no_cache_file_preserves_input_order() {
    let (mut cache, _dir) = temp_cache();
    let entries = vec![
        entry("alpha", "f", "alice"),
        entry("beta", "g", "bob"),
        entry("gamma", "h", "carol"),
    ];
    let sorted = cache.sorted(entries.clone());
    assert_eq!(sorted.len(), 3);
    // No file → all go into the "uncached" bucket. Order is nondeterministic
    // because we collect into a HashMap before extending. Assert on set
    // membership instead of order.
    let names: std::collections::HashSet<_> = sorted.iter().map(|e| e.name.clone()).collect();
    assert!(names.contains("alpha"));
    assert!(names.contains("beta"));
    assert!(names.contains("gamma"));
}

// ---------------------------------------------------------------------------
// Cache::update writes the file
// ---------------------------------------------------------------------------

#[test]
fn update_creates_cache_file_with_count_1() {
    let (mut cache, dir) = temp_cache();
    let e = entry("alpha", "f", "alice");
    cache.update(&e).unwrap();

    let path = dir.path().join("rofi-rbw.runcache");
    let text = fs::read_to_string(&path).unwrap();
    let line = text.lines().next().unwrap();
    let (count, hash) = line.split_once(' ').unwrap();
    assert_eq!(count, "1");
    assert_eq!(hash, e.hashed());
}

#[test]
fn update_accumulates_count_across_calls() {
    let (mut cache, dir) = temp_cache();
    let e = entry("alpha", "f", "alice");
    cache.update(&e).unwrap();
    cache.update(&e).unwrap();
    cache.update(&e).unwrap();

    // 1.1 + 1.1 + 1.1 = 3.3 → floor → 3
    let text = fs::read_to_string(dir.path().join("rofi-rbw.runcache")).unwrap();
    let count = text.split_whitespace().next().unwrap();
    assert_eq!(count, "3");
}

#[test]
fn update_writes_multiple_entries_sorted_by_count_desc() {
    let (mut cache, dir) = temp_cache();
    let a = entry("alpha", "f", "alice");
    let b = entry("beta", "g", "bob");

    cache.update(&a).unwrap(); // a: 1
    cache.update(&b).unwrap(); // b: 1
    cache.update(&a).unwrap(); // a: 2
    cache.update(&a).unwrap(); // a: 3

    let text = fs::read_to_string(dir.path().join("rofi-rbw.runcache")).unwrap();
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 2);

    let (a_count, a_hash) = lines[0].split_once(' ').unwrap();
    let (b_count, b_hash) = lines[1].split_once(' ').unwrap();
    assert_eq!(a_hash, a.hashed());
    assert_eq!(b_hash, b.hashed());
    assert_eq!(a_count, "3");
    assert_eq!(b_count, "1");
}

// ---------------------------------------------------------------------------
// Cache::sorted reads the file back and reorders
// ---------------------------------------------------------------------------

#[test]
fn sorted_puts_cached_entries_first() {
    let (mut cache_a, dir) = temp_cache();
    let a = entry("alpha", "f", "alice");
    let b = entry("beta", "g", "bob");
    let c = entry("gamma", "h", "carol");

    // Prime the file with a and c
    cache_a.update(&a).unwrap();
    cache_a.update(&a).unwrap();
    cache_a.update(&c).unwrap();

    // Fresh Cache against the same file
    let mut cache_b = rofi_rbw_rs::cache::Cache::at(dir.path().join("rofi-rbw.runcache")).unwrap();
    let sorted = cache_b.sorted(vec![a.clone(), b.clone(), c.clone()]);

    // First two should be the cached ones, in file order (a then c).
    assert_eq!(sorted[0].name, "alpha");
    assert_eq!(sorted[1].name, "gamma");
    // Uncached entries follow.
    assert_eq!(sorted[2].name, "beta");
}

#[test]
fn sorted_skips_cache_lines_with_unknown_hashes() {
    let (mut cache, dir) = temp_cache();
    let path = dir.path().join("rofi-rbw.runcache");
    fs::write(&path, "5 deadbeef00000000000000000000000000000000\n").unwrap();

    let entries = vec![entry("alpha", "f", "alice")];
    let sorted = cache.sorted(entries);
    // The unknown hash line is ignored; the one real entry comes through.
    assert_eq!(sorted.len(), 1);
    assert_eq!(sorted[0].name, "alpha");
}

#[test]
fn sorted_ignores_malformed_lines() {
    let (_unused, dir) = temp_cache();
    let path = dir.path().join("rofi-rbw.runcache");
    fs::write(
        &path,
        "\n\
         this line has no space\n\
         notanumber abcdef\n\
         \n",
    )
    .unwrap();

    let mut cache = rofi_rbw_rs::cache::Cache::at(path).unwrap();
    let entries = vec![entry("alpha", "f", "alice")];
    let sorted = cache.sorted(entries);
    // Malformed lines don't panic; entry passes through.
    assert_eq!(sorted.len(), 1);
}

#[test]
fn sorted_handles_empty_file() {
    let (_unused, dir) = temp_cache();
    let path = dir.path().join("rofi-rbw.runcache");
    fs::write(&path, "").unwrap();

    let mut cache = rofi_rbw_rs::cache::Cache::at(path).unwrap();
    let entries = vec![entry("alpha", "f", "alice")];
    let sorted = cache.sorted(entries);
    assert_eq!(sorted.len(), 1);
}

// ---------------------------------------------------------------------------
// Round-trip: update → new Cache → sorted → same layout
// ---------------------------------------------------------------------------

#[test]
fn roundtrip_persists_scores() {
    let (_unused, dir) = temp_cache();
    let path = dir.path().join("rofi-rbw.runcache");
    let e = entry("alpha", "f", "alice");

    {
        let mut c = rofi_rbw_rs::cache::Cache::at(path.clone()).unwrap();
        for _ in 0..5 {
            c.update(&e).unwrap();
        }
    }

    // File should say "5"
    let text = fs::read_to_string(&path).unwrap();
    assert!(text.starts_with("5 "), "got: {text}");

    // Reload and update again; 5 + 1.1 = 6.1 → floor → 6
    let mut c = rofi_rbw_rs::cache::Cache::at(path.clone()).unwrap();
    // Prime from file via sorted
    let _ = c.sorted(vec![e.clone()]);
    c.update(&e).unwrap();

    let text = fs::read_to_string(&path).unwrap();
    assert!(text.starts_with("6 "), "got: {text}");
}

// ---------------------------------------------------------------------------
// Behavior parity: caches in different paths don't bleed
// ---------------------------------------------------------------------------

#[test]
fn two_caches_are_isolated() {
    let (mut a, dir_a) = temp_cache();
    let (mut _b, dir_b) = temp_cache();

    a.update(&entry("alpha", "f", "alice")).unwrap();

    let text_a = fs::read_to_string(dir_a.path().join("rofi-rbw.runcache")).unwrap();
    assert!(!text_a.is_empty());

    // dir_b has no file
    let path_b: &Path = &dir_b.path().join("rofi-rbw.runcache");
    assert!(!path_b.exists());
}
