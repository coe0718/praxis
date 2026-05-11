//! Response cache — cache LLM responses by input hash to reduce costs and latency.
//!
//! Uses content-addressable hashing: same input → same cached response.

use std::collections::HashMap;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// A cached response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedResponse {
    /// Hash of the input that produced this response.
    pub input_hash: String,
    /// The cached output text.
    pub output: String,
    /// Model used.
    pub model: String,
    /// Input token count.
    pub input_tokens: u64,
    /// Output token count.
    pub output_tokens: u64,
    /// When cached.
    pub cached_at: chrono::DateTime<chrono::Utc>,
    /// How long until this expires (seconds).
    pub ttl_seconds: u64,
}

impl CachedResponse {
    pub fn is_expired(&self) -> bool {
        let age = chrono::Utc::now() - self.cached_at;
        age.num_seconds() > self.ttl_seconds as i64
    }
}

/// Response cache with TTL support.
pub struct ResponseCache {
    entries: Mutex<HashMap<String, CachedResponse>>,
    hit_count: Mutex<u64>,
    miss_count: Mutex<u64>,
    default_ttl: u64,
}

impl ResponseCache {
    pub fn new(default_ttl_seconds: u64) -> Self {
        Self {
            entries: Mutex::new(HashMap::new()),
            hit_count: Mutex::new(0),
            miss_count: Mutex::new(0),
            default_ttl: default_ttl_seconds,
        }
    }

    /// Compute hash of input text + model.
    pub fn compute_hash(input: &str, model: &str) -> String {
        let mut hasher = Sha256::new();
        // Use Digest::update which works with any input that implements AsRef<[u8]>
        hasher.update(input.as_bytes());
        hasher.update(model.as_bytes());
        hex::encode(hasher.finalize())
    }

    /// Get a cached response if available and not expired.
    pub fn get(&self, input: &str, model: &str) -> Option<CachedResponse> {
        let hash = Self::compute_hash(input, model);
        let mut entries = self.entries.lock().unwrap();

        if let Some(resp) = entries.get(&hash) {
            if !resp.is_expired() {
                *self.hit_count.lock().unwrap() += 1;
                return Some(resp.clone());
            }
            // Remove expired
            entries.remove(&hash);
        }

        *self.miss_count.lock().unwrap() += 1;
        None
    }

    /// Store a response in the cache.
    pub fn put(
        &self,
        input: &str,
        model: &str,
        output: String,
        input_tokens: u64,
        output_tokens: u64,
    ) {
        let hash = Self::compute_hash(input, model);
        let entry = CachedResponse {
            input_hash: hash.clone(),
            output,
            model: model.to_string(),
            input_tokens,
            output_tokens,
            cached_at: chrono::Utc::now(),
            ttl_seconds: self.default_ttl,
        };

        self.entries.lock().unwrap().insert(hash, entry);
    }

    /// Remove a specific entry.
    pub fn invalidate(&self, input: &str, model: &str) -> bool {
        let hash = Self::compute_hash(input, model);
        self.entries.lock().unwrap().remove(&hash).is_some()
    }

    /// Clear all entries.
    pub fn clear(&self) {
        self.entries.lock().unwrap().clear();
    }

    /// Remove expired entries.
    pub fn cleanup(&self) {
        let mut entries = self.entries.lock().unwrap();
        entries.retain(|_, v| !v.is_expired());
    }

    /// Number of cached entries.
    pub fn len(&self) -> usize {
        self.entries.lock().unwrap().len()
    }

    /// Whether cache is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.lock().unwrap().is_empty()
    }

    /// Cache hit rate.
    pub fn hit_rate(&self) -> f64 {
        let hits = *self.hit_count.lock().unwrap();
        let misses = *self.miss_count.lock().unwrap();
        let total = hits + misses;
        if total == 0 { 0.0 } else { hits as f64 / total as f64 }
    }

    /// Get hit/miss counts.
    pub fn stats(&self) -> (u64, u64) {
        (*self.hit_count.lock().unwrap(), *self.miss_count.lock().unwrap())
    }

    /// Reset hit/miss counters.
    pub fn reset_stats(&self) {
        *self.hit_count.lock().unwrap() = 0;
        *self.miss_count.lock().unwrap() = 0;
    }

    /// Estimated token savings.
    pub fn tokens_saved(&self) -> u64 {
        let entries = self.entries.lock().unwrap();
        entries.values().map(|e| e.output_tokens).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_deterministic() {
        let h1 = ResponseCache::compute_hash("hello", "gpt-4o");
        let h2 = ResponseCache::compute_hash("hello", "gpt-4o");
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_hash_different_inputs() {
        let h1 = ResponseCache::compute_hash("hello", "gpt-4o");
        let h2 = ResponseCache::compute_hash("world", "gpt-4o");
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_hash_different_models() {
        let h1 = ResponseCache::compute_hash("hello", "gpt-4o");
        let h2 = ResponseCache::compute_hash("hello", "gpt-4o-mini");
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_cache_miss() {
        let cache = ResponseCache::new(3600);
        let result = cache.get("hello", "gpt-4o");
        assert!(result.is_none());
    }

    #[test]
    fn test_cache_hit() {
        let cache = ResponseCache::new(3600);
        cache.put("hello", "gpt-4o", "Hi there!".to_string(), 2, 3);

        let result = cache.get("hello", "gpt-4o");
        assert!(result.is_some());
        assert_eq!(result.unwrap().output, "Hi there!");
    }

    #[test]
    fn test_cache_different_models() {
        let cache = ResponseCache::new(3600);
        cache.put("hello", "gpt-4o", "Hi".to_string(), 2, 2);

        // Different model = different cache entry
        let result = cache.get("hello", "gpt-4o-mini");
        assert!(result.is_none());
    }

    #[test]
    fn test_invalidate() {
        let cache = ResponseCache::new(3600);
        cache.put("hello", "gpt-4o", "Hi".to_string(), 2, 2);

        assert!(cache.get("hello", "gpt-4o").is_some());

        cache.invalidate("hello", "gpt-4o");
        assert!(cache.get("hello", "gpt-4o").is_none());
    }

    #[test]
    fn test_hit_rate() {
        let cache = ResponseCache::new(3600);
        cache.put("a", "gpt-4o", "1".to_string(), 1, 1);

        assert_eq!(cache.hit_rate(), 0.0);

        cache.get("a", "gpt-4o");
        assert_eq!(cache.hit_rate(), 1.0);

        cache.get("b", "gpt-4o");
        assert!((cache.hit_rate() - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_stats() {
        let cache = ResponseCache::new(3600);
        cache.put("a", "gpt-4o", "1".to_string(), 1, 1);

        cache.get("a", "gpt-4o");
        cache.get("b", "gpt-4o");

        let (hits, misses) = cache.stats();
        assert_eq!(hits, 1);
        assert_eq!(misses, 1);
    }

    #[test]
    fn test_tokens_saved() {
        let cache = ResponseCache::new(3600);
        cache.put("a", "gpt-4o", "response1".to_string(), 10, 100);
        cache.put("b", "gpt-4o", "response2".to_string(), 10, 200);

        assert_eq!(cache.tokens_saved(), 300);
    }

    #[test]
    fn test_len() {
        let cache = ResponseCache::new(3600);
        assert_eq!(cache.len(), 0);

        cache.put("a", "gpt-4o", "1".to_string(), 1, 1);
        assert_eq!(cache.len(), 1);

        cache.put("b", "gpt-4o", "2".to_string(), 1, 1);
        assert_eq!(cache.len(), 2);
    }
}
