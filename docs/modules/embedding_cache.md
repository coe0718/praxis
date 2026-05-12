# Embedding Cache

> Cache vector embeddings locally to reduce LLM API calls and lower costs.

## Overview

The embedding cache module provides a simple on-disk cache for embedding vectors with TTL-based expiration. By caching embeddings locally, Praxis avoids redundant LLM API calls for already-embedded text, achieving up to 50% cost reduction on embedding operations (Moltis feature).

The cache persists to a JSON file (`embedding_cache.json`) in the data directory and supports load, save, get, put, and cleanup operations. Entries are automatically evicted when their TTL expires.

## Architecture

### Key Types

| Type | Description |
|------|-------------|
| `EmbeddingCacheEntry` | A single cached vector with its creation timestamp and TTL. |
| `EmbeddingCache` | The main cache manager holding entries in a `HashMap<String, EmbeddingCacheEntry>` and managing persistence. |

### Relationships

`EmbeddingCache` owns a `HashMap` of entries keyed by string (typically a text hash or identifier). The `cache_file` path is derived from `PraxisPaths::data_dir`.

## Public API

### `EmbeddingCacheEntry`

```rust
pub struct EmbeddingCacheEntry {
    pub vector: Vec<f32>,
    pub cached_at: chrono::DateTime<chrono::Utc>,
    pub ttl_seconds: u64,
}
```

The cache entry holds the embedding vector, the timestamp when it was cached, and the TTL in seconds.

### `EmbeddingCache`

```rust
impl EmbeddingCache {
    pub fn new(paths: &PraxisPaths) -> Self
    pub fn load(&mut self) -> Result<()>
    pub fn save(&self) -> Result<()>
    pub fn get(&mut self, key: &str) -> Option<EmbeddingCacheEntry>
    pub fn put(&mut self, key: String, vector: Vec<f32>, ttl_seconds: u64)
    pub fn cleanup(&mut self)
}
```

- **`new`** — Creates a new empty cache with the cache file path set to `{data_dir}/embedding_cache.json`.
- **`load`** — Reads the cache from disk. No-op if the cache file doesn't exist.
- **`save`** — Persists the current cache entries to disk as pretty-printed JSON.
- **`get`** — Retrieves a cached embedding by key. Returns `None` if the entry is missing or expired (expired entries are automatically removed).
- **`put`** — Inserts a new embedding vector into the cache with the specified TTL.
- **`cleanup`** — Removes all expired entries from the in-memory cache.

## Configuration

No direct configuration fields. The cache file path is derived from `PraxisPaths`:

```toml
# The cache file is auto-located:
# data_dir/embedding_cache.json
```

## Usage

```rust
use praxis::embedding_cache::EmbeddingCache;

let mut cache = EmbeddingCache::new(&paths);
cache.load()?;

// Store an embedding with 1-hour TTL
cache.put("text-hash-123".into(), vector, 3600);

// Retrieve (returns None if expired)
if let Some(entry) = cache.get("text-hash-123") {
    println!("Vector: {:?}", entry.vector);
}
```

## Data Files

| File | Purpose |
|------|---------|
| `data_dir/embedding_cache.json` | Persistent JSON cache of embedding vectors with timestamps and TTLs. |

## Dependencies

- **`paths`** — Uses `PraxisPaths::data_dir` to locate the cache file.
- **`serde` / `serde_json`** — Serialization of cache entries to/from JSON.
- **`chrono`** — Timestamp tracking for TTL-based expiration.

## Source

`src/embedding_cache.rs`