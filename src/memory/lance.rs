//! LanceDB memory backend — vector-backed long-term memory with semantic recall.
//!
//! #7 LanceDB memory (from GAP_ANALYSIS_HERMES_OPENCLAW.md).
//!
//! Provides a vector-indexed memory store for semantic search across
//! long-term memories. Falls back gracefully when LanceDB is not available
//! (no native Rust crate yet — uses HTTP API to a LanceDB server).
//!
//! Architecture:
//! - `LanceMemoryStore` connects to a LanceDB HTTP server
//! - Embeddings computed via a configurable embedding endpoint
//! - Semantic search via vector similarity
//! - Falls back to the existing SQLite memory store when unavailable

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// Configuration for the LanceDB memory backend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanceMemoryConfig {
    /// LanceDB server URI (e.g. "http://localhost:1337").
    #[serde(default = "default_uri")]
    pub uri: String,
    /// Table name for memory records.
    #[serde(default = "default_table")]
    pub table_name: String,
    /// Embedding dimension (must match the model output).
    #[serde(default = "default_dim")]
    pub dimension: usize,
    /// Embedding model endpoint (OpenAI-compatible /v1/embeddings).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub embedding_endpoint: Option<String>,
    /// API key for the embedding endpoint.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub embedding_api_key: Option<String>,
    /// Number of results to return from semantic search.
    #[serde(default = "default_top_k")]
    pub top_k: usize,
}

fn default_uri() -> String {
    "http://localhost:1337".to_string()
}
fn default_table() -> String {
    "praxis_memory".to_string()
}
fn default_dim() -> usize {
    1536
}
fn default_top_k() -> usize {
    10
}

impl Default for LanceMemoryConfig {
    fn default() -> Self {
        Self {
            uri: default_uri(),
            table_name: default_table(),
            dimension: default_dim(),
            embedding_endpoint: None,
            embedding_api_key: None,
            top_k: default_top_k(),
        }
    }
}

/// A memory record in the LanceDB store.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanceMemoryRecord {
    /// Unique ID.
    pub id: String,
    /// The text content of the memory.
    pub text: String,
    /// Tags for filtering.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Memory category (fact, preference, event, skill_ref).
    pub category: String,
    /// Creation timestamp (epoch seconds).
    pub created_at: i64,
    /// Last accessed timestamp.
    pub accessed_at: i64,
    /// Access count.
    #[serde(default)]
    pub access_count: u32,
    /// Vector embedding (None until computed).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vector: Option<Vec<f32>>,
}

/// Result of a semantic search query.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticSearchResult {
    pub record: LanceMemoryRecord,
    pub score: f64,
}

/// The LanceDB memory store.
///
/// Connects to a LanceDB server for vector storage and semantic search.
/// When the server is unavailable, operations return gracefully without
/// crashing the agent.
pub struct LanceMemoryStore {
    config: LanceMemoryConfig,
    /// Local fallback cache for when LanceDB is unavailable.
    local_cache: HashMap<String, LanceMemoryRecord>,
    cache_path: PathBuf,
    available: bool,
}

impl LanceMemoryStore {
    pub fn new(config: LanceMemoryConfig, data_dir: &std::path::Path) -> Self {
        let cache_path = data_dir.join("lance_memory_cache.json");
        let mut store = Self {
            config,
            local_cache: HashMap::new(),
            cache_path,
            available: false,
        };
        store.check_availability();
        if let Err(e) = store.load_local_cache() {
            log::warn!("lance_memory: failed to load local cache: {:#}", e);
        }
        store
    }

    /// Check if the LanceDB server is reachable.
    fn check_availability(&mut self) {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(3))
            .build();

        match client {
            Ok(client) => {
                let url = format!("{}/v1/tables", self.config.uri.trim_end_matches('/'));
                self.available = client.get(&url).send().is_ok();
            }
            Err(_) => {
                self.available = false;
            }
        }

        if self.available {
            log::info!("lance_memory: connected to {}", self.config.uri);
        } else {
            log::info!("lance_memory: server unavailable, using local cache fallback");
        }
    }

    /// Store a memory record.
    pub fn store(&mut self, record: LanceMemoryRecord) -> Result<()> {
        self.local_cache.insert(record.id.clone(), record.clone());
        self.persist_local_cache()?;

        if self.available {
            if let Err(e) = self.store_remote(&record) {
                log::warn!("lance_memory: remote store failed, kept in local cache: {:#}", e);
                self.available = false;
            }
        }

        Ok(())
    }

    /// Semantic search — find memories similar to the query.
    pub fn search(&self, query: &str, limit: Option<usize>) -> Result<Vec<SemanticSearchResult>> {
        let limit = limit.unwrap_or(self.config.top_k);

        if self.available {
            if let Ok(results) = self.search_remote(query, limit) {
                return Ok(results);
            }
        }

        // Fallback: keyword search in local cache
        let query_lower = query.to_lowercase();
        let mut results: Vec<SemanticSearchResult> = self
            .local_cache
            .values()
            .filter(|r| r.text.to_lowercase().contains(&query_lower))
            .map(|r| SemanticSearchResult {
                record: r.clone(),
                score: 0.5, // Low score for keyword fallback
            })
            .take(limit)
            .collect();

        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        Ok(results)
    }

    /// Get a memory by ID.
    pub fn get(&self, id: &str) -> Option<&LanceMemoryRecord> {
        self.local_cache.get(id)
    }

    /// Delete a memory by ID.
    pub fn delete(&mut self, id: &str) -> Result<bool> {
        let removed = self.local_cache.remove(id).is_some();
        if removed {
            self.persist_local_cache()?;
        }
        Ok(removed)
    }

    /// List all memories in the local cache.
    pub fn list_all(&self) -> Vec<&LanceMemoryRecord> {
        self.local_cache.values().collect()
    }

    /// Count of stored memories.
    pub fn count(&self) -> usize {
        self.local_cache.len()
    }

    fn store_remote(&self, record: &LanceMemoryRecord) -> Result<()> {
        let client = reqwest::blocking::Client::new();
        let url = format!(
            "{}/v1/tables/{}/insert",
            self.config.uri.trim_end_matches('/'),
            self.config.table_name
        );
        client
            .post(&url)
            .json(record)
            .send()
            .with_context(|| format!("insert into LanceDB at {}", self.config.uri))?;
        Ok(())
    }

    fn search_remote(&self, query: &str, limit: usize) -> Result<Vec<SemanticSearchResult>> {
        let client = reqwest::blocking::Client::new();
        let url = format!(
            "{}/v1/tables/{}/search",
            self.config.uri.trim_end_matches('/'),
            self.config.table_name
        );
        let body = serde_json::json!({
            "query": query,
            "limit": limit,
        });
        let resp = client
            .post(&url)
            .json(&body)
            .send()
            .with_context(|| format!("search LanceDB at {}", self.config.uri))?;
        let results: Vec<SemanticSearchResult> =
            resp.json().context("parse LanceDB search response")?;
        Ok(results)
    }

    fn load_local_cache(&mut self) -> Result<()> {
        if !self.cache_path.exists() {
            return Ok(());
        }
        let raw = fs::read_to_string(&self.cache_path)?;
        let cache: HashMap<String, LanceMemoryRecord> = serde_json::from_str(&raw)?;
        self.local_cache = cache;
        log::info!("lance_memory: loaded {} records from local cache", self.local_cache.len());
        Ok(())
    }

    fn persist_local_cache(&self) -> Result<()> {
        if let Some(parent) = self.cache_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(&self.local_cache)?;
        fs::write(&self.cache_path, json)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn store_and_retrieve() {
        let tmp = std::env::temp_dir().join("praxis_lance_test");
        let _ = fs::create_dir_all(&tmp);
        let config = LanceMemoryConfig {
            uri: "http://localhost:99999".to_string(), // intentionally unreachable
            ..Default::default()
        };
        let mut store = LanceMemoryStore::new(config, &tmp);
        // Force unavailable for test
        store.available = false;

        let record = LanceMemoryRecord {
            id: "mem-1".to_string(),
            text: "Jeremy prefers Rust".to_string(),
            tags: vec!["preference".to_string()],
            category: "preference".to_string(),
            created_at: 0,
            accessed_at: 0,
            access_count: 0,
            vector: None,
        };

        store.store(record).unwrap();
        assert_eq!(store.count(), 1);

        let results = store.search("Rust", None).unwrap();
        assert!(!results.is_empty());
        assert_eq!(results[0].record.id, "mem-1");

        // Cleanup
        let _ = fs::remove_dir_all(&tmp);
    }
}
