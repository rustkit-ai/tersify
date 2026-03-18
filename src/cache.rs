//! Incremental compression cache.
//!
//! Files whose content + options hash have already been compressed are served
//! from a disk cache at `~/.tersify/cache/`, avoiding redundant work on
//! repeated `tersify src/` runs over unchanged files.
//!
//! The cache is keyed by a 64-bit hash of the content and option flags.
//! Collisions are extremely unlikely for a dev-tool cache and the consequence
//! is merely a wrong cached result — the next run on changed content will
//! produce a fresh entry.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;

fn cache_dir() -> PathBuf {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".tersify").join("cache")
}

fn cache_key(content: &str, opts: u8) -> String {
    let mut h = DefaultHasher::new();
    content.hash(&mut h);
    opts.hash(&mut h);
    format!("{:016x}", h.finish())
}

/// Retrieve a previously cached compression result.
///
/// `opts` encodes the [`crate::compress::CompressOptions`] flags as a bitmask:
/// bit 0 = ast, bit 1 = strip_docs, bit 2 = smart.
pub fn get(content: &str, opts: u8) -> Option<String> {
    std::fs::read_to_string(cache_dir().join(cache_key(content, opts))).ok()
}

/// Store a compressed result in the cache.
pub fn set(content: &str, opts: u8, compressed: &str) {
    let dir = cache_dir();
    let _ = std::fs::create_dir_all(&dir);
    let _ = std::fs::write(dir.join(cache_key(content, opts)), compressed);
}

/// Evict all cached entries older than `max_age_days` days.
pub fn evict_old(max_age_days: u64) {
    let dir = cache_dir();
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return;
    };
    let cutoff = std::time::SystemTime::now()
        - std::time::Duration::from_secs(max_age_days * 86_400);
    for entry in entries.flatten() {
        if let Ok(meta) = entry.metadata()
            && let Ok(modified) = meta.modified()
            && modified < cutoff
        {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}
