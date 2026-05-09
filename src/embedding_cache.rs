//! Embedding cache — Cache vector embeddings to reduce LLM API calls.
//!
//! Moltis feature: 50% cost reduction by caching embeddings locally.

use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::paths::PraxisPaths;

/// Cache entry for an embedding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingCacheEntry {
    /// The cached embedding vector.
    pub vector: Vec<f32>,
    /// When it was cached.
    pub cached_at: chrono::DateTime<chrono::Utc>,
    /// TTL in seconds.
    pub ttl_seconds: u64,
}

/// Embedding cache with TTL support.
pub struct EmbeddingCache {
    cache_file: PathBuf,
    entries: HashMap<String, EmbeddingCacheEntry>,
}

impl EmbeddingCache {
    pub fn new(paths: &PraxisPaths) -> Self {
        Self {
            cache_file: paths.data_dir.join("embedding_cache.json"),
            entries: HashMap::new(),
        }
    }

    /// Load cache from disk.
    pub fn load(&mut self) -> Result<()> {
        if !self.cache_file.exists() {
            return Ok(());
        }
        let json = std::fs::read_to_string(&self.cache_file)?;
        self.entries = serde_json::from_str(&json)?;
        Ok(())
    }

    /// Save cache to disk.
    pub fn save(&self) -> Result<()> {
        let json = serde_json::to_string_pretty(&self.entries)?;
        std::fs::write(&self.cache_file, json)?;
        Ok(())
    }

    /// Get cached embedding if not expired.
    pub fn get(&self, key: &str) -> Option<&EmbeddingCacheEntry> {
        self.entries.get(key).and_then(|e| {
            let now: chrono::DateTime<chrono::Utc> = chrono::Utc::now();
            if (now - e.cached_at).num_seconds() < e.ttl_seconds as i64 {
                Some(e)
            } else {
                None
            }
        })
    }

    /// Store an embedding in cache.
    pub fn put(&mut self, key: String, vector: Vec<f32>, ttl_seconds: u64) {
        self.entries.insert(
            key,
            EmbeddingCacheEntry {
                vector,
                cached_at: chrono::Utc::now(),
                ttl_seconds,
            },
        );
    }

    /// Clear expired entries.
    pub fn cleanup(&mut self) {
        let now: chrono::DateTime<chrono::Utc> = chrono::Utc::now();
        self.entries
            .retain(|_, e| (now - e.cached_at).num_seconds() < e.ttl_seconds as i64);
    }
}
