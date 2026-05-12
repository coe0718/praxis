# Response Cache

> Cache LLM responses by input hash to reduce costs and latency — content-addressable caching with TTL support.

## Overview

The response cache module provides content-addressable caching for LLM responses and HTTP responses. Each entry is keyed by a SHA-256 hash of the input text + model name, ensuring that identical inputs to the same model return cached results. Entries have a configurable TTL (time-to-live), and expired entries are opportunistically swept on cache miss.

A separate global HTTP response cache (`HTTP_CACHE`) is provided as a `Lazy<Mutex<ResponseCache>>` for caching raw HTTP tool responses. The module tracks hit/miss counts and provides estimated token savings.

**Current status:** Fully implemented. Used in the LLM backend for caching completions and in the HTTP tool executor for caching GET/POST responses.

## Architecture

### Key Types

| Type | Description |
|------|-------------|
| `CachedResponse` | Single cache entry: hash, output, model, token counts, timestamp, TTL. |
| `ResponseCache` | In-memory cache with `HashMap<String, CachedResponse>` behind a Mutex, plus hit/miss counters. |
| `HTTP_CACHE` | Global `Lazy<Mutex<ResponseCache>>` for shared HTTP response caching (5-min default TTL). |

### Key Pattern

Hashing is deterministic: `SHA-256(input_bytes + model_bytes)` → hex string. Two calls with identical input + model produce the same hash. Different models produce different cache entries even for the same input.

## Public API

### `CachedResponse`

```rust
pub struct CachedResponse {
    pub input_hash: String,
    pub output: String,
    pub model: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cached_at: chrono::DateTime<chrono::Utc>,
    pub ttl_seconds: u64,
}

impl CachedResponse {
    pub fn is_expired(&self) -> bool
}
```

### `ResponseCache`

```rust
impl ResponseCache {
    pub fn new(default_ttl_seconds: u64) -> Self
    pub fn compute_hash(input: &str, model: &str) -> String
    pub fn get(&self, input: &str, model: &str) -> Option<CachedResponse>
    pub fn put(&self, input: &str, model: &str, output: String, input_tokens: u64, output_tokens: u64)
    pub fn invalidate(&self, input: &str, model: &str) -> bool
    pub fn clear(&self)
    pub fn cleanup(&self)
    pub fn len(&self) -> usize
    pub fn is_empty(&self) -> bool
    pub fn hit_rate(&self) -> f64
    pub fn put_http(&self, key: &str, body: String, ttl_secs: u64)
    pub fn stats(&self) -> (u64, u64)
    pub fn tokens_saved(&self) -> u64
}
```

- **`compute_hash`** — Static method: SHA-256 of input + model bytes → hex.
- **`get`** — Looks up by hash; returns `Some` if found and not expired; on miss, opportunistically sweeps all expired entries.
- **`put`** — Stores a response with the default TTL.
- **`invalidate`** — Removes a specific entry; returns `true` if existed.
- **`hit_rate`** — Returns `hits / (hits + misses)`.
- **`put_http`** — Stores raw HTTP body keyed by pre-computed hash.
- **`tokens_saved`** — Sum of `output_tokens` for all cached entries.

### Global HTTP Cache Functions

```rust
pub fn get_cached(key: &str, ttl_secs: u64) -> Option<String>
pub fn put_cached(key: &str, body: String, ttl_secs: u64)
pub fn http_cache_stats() -> (u64, u64)
```

- **`get_cached`** — Look up cached HTTP response by pre-computed key.
- **`put_cached`** — Store HTTP response in global cache.
- **`http_cache_stats`** — Return (hits, misses) for the global cache.

## Configuration

No `praxis.toml` fields currently. Configured programmatically:

```rust
use praxis::response_cache::ResponseCache;

// Create with 1-hour default TTL
let cache = ResponseCache::new(3600);

// Cache an LLM response
cache.put("Hello, how are you?", "gpt-4o", "I'm doing well!", 5, 4);

// Later — retrieve
if let Some(response) = cache.get("Hello, how are you?", "gpt-4o") {
    println!("Cached: {}", response.output);
}

// Use global HTTP cache
use praxis::response_cache::{put_cached, get_cached};
put_cached("sha256-of-url", "response body".to_string(), 300);
if let Some(body) = get_cached("sha256-of-url", 300) {
    // Use cached response
}
```

## Dependencies

- **`sha2`** — SHA-256 hashing for content-addressable keys.
- **`hex`** — Binary-to-hex encoding.
- **`chrono`** — Timestamps and TTL expiry.
- **`once_cell`** — `Lazy` initialization for the global HTTP cache.

## Status

- ✅ Content-addressable hashing (SHA-256 of input + model)
- ✅ TTL-based expiry with stale entry cleanup
- ✅ Hit/miss counters and hit rate calculation
- ✅ Token savings estimation
- ✅ Public API (get, put, invalidate, clear)
- ✅ Global HTTP response cache with free functions
- ✅ Comprehensive test coverage

## Source

`src/response_cache.rs`