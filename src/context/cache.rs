//! Persistent context cache — stores a compressed working set from the last
//! Reflect phase so warm-start Orient runs skip expensive re-loading.
//!
//! The cache is a small JSON file (`context_cache.json`) that holds a handful
//! of high-value excerpts with their source labels.  Orient loads it as an
//! additional context source before running normal source collection.  Reflect
//! writes it at the end of each successful session.
//!
//! Cache entries expire after `CACHE_TTL_HOURS` hours so stale data cannot
//! crowd out fresh information.

use std::{fs, path::Path};

use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

/// How long a cache entry stays valid without being refreshed.
const CACHE_TTL_HOURS: i64 = 48;

/// A single cached excerpt from a named context source.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextCacheEntry {
    /// Human-readable label (e.g. "hot_memories", "goals", "journal").
    pub source: String,
    /// The compressed / excerpted content.
    pub content: String,
    /// Estimated token cost of `content`.
    pub token_estimate: u32,
}

/// The full cache written to `context_cache.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextCache {
    pub entries: Vec<ContextCacheEntry>,
    /// When this cache was written.
    pub written_at: DateTime<Utc>,
    /// Session ID of the Reflect run that produced this cache (informational).
    pub session_id: Option<i64>,
}

impl ContextCache {
    pub fn new(entries: Vec<ContextCacheEntry>, session_id: Option<i64>) -> Self {
        Self {
            entries,
            written_at: Utc::now(),
            session_id,
        }
    }

    /// True if the cache is young enough to be useful.
    pub fn is_fresh(&self, now: DateTime<Utc>) -> bool {
        now - self.written_at < Duration::hours(CACHE_TTL_HOURS)
    }

    /// Total estimated token cost of all entries.
    pub fn total_tokens(&self) -> u32 {
        self.entries.iter().map(|e| e.token_estimate).sum()
    }
}

/// Write the cache to disk, replacing any prior version.
pub fn write_context_cache(path: &Path, cache: &ContextCache) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    let raw = serde_json::to_string_pretty(cache).context("failed to serialize context cache")?;
    fs::write(path, raw).with_context(|| format!("failed to write {}", path.display()))
}

/// Load the cache from disk.  Returns `None` if the file is missing or stale.
pub fn load_context_cache(path: &Path, now: DateTime<Utc>) -> Option<ContextCache> {
    let raw = fs::read_to_string(path).ok()?;
    let cache: ContextCache = serde_json::from_str(&raw).ok()?;
    if cache.is_fresh(now) { Some(cache) } else { None }
}

/// Format the cache as a human-readable context block for injection into Orient.
pub fn render_context_cache(cache: &ContextCache) -> String {
    let mut out = format!(
        "## Cached context (from last session, {} entries, ~{} tokens)\n\n",
        cache.entries.len(),
        cache.total_tokens(),
    );
    for entry in &cache.entries {
        out.push_str(&format!("### {}\n{}\n\n", entry.source, entry.content));
    }
    out
}
