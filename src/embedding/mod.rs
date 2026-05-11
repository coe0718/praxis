//! Embedding provider system — generate vector embeddings via multiple providers.
//!
//! Supports OpenAI (text-embedding-3-small/large), local providers, and cached lookups.
//! Configure via `praxis.toml`:
//!
//! ```toml
//! [embedding]
//! provider = "openai"  # openai, local
//! model = "text-embedding-3-small"
//! dimensions = 1536    # optional dimension reduction
//! cache_ttl = 86400    # 24h cache TTL
//! ```

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::embedding_cache::EmbeddingCache;
use crate::paths::PraxisPaths;

// ── Configuration ─────────────────────────────────────────────────────────────

/// Embedding provider configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingConfig {
    #[serde(default = "default_provider")]
    pub provider: String,
    #[serde(default = "default_model")]
    pub model: String,
    /// Output dimensions (optional — model default if not set).
    pub dimensions: Option<usize>,
    /// Cache TTL in seconds (default: 24h).
    #[serde(default = "default_cache_ttl")]
    pub cache_ttl: u64,
}

fn default_provider() -> String {
    "openai".to_string()
}
fn default_model() -> String {
    "text-embedding-3-small".to_string()
}
fn default_cache_ttl() -> u64 {
    86400
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            provider: default_provider(),
            model: default_model(),
            dimensions: None,
            cache_ttl: default_cache_ttl(),
        }
    }
}

impl EmbeddingConfig {
    /// Load from praxis.toml [embedding] section.
    pub fn load(paths: &PraxisPaths) -> Result<Self> {
        let config_path = &paths.config_file;
        if !config_path.exists() {
            return Ok(Self::default());
        }
        let raw = std::fs::read_to_string(config_path).context("read praxis.toml")?;
        let doc: toml::Value = toml::from_str(&raw).context("parse praxis.toml")?;
        let section = doc.get("embedding").and_then(|v| v.as_table()).cloned();

        let mut cfg = Self::default();
        if let Some(t) = section {
            if let Some(v) = t.get("provider").and_then(|v| v.as_str()) {
                cfg.provider = v.to_string();
            }
            if let Some(v) = t.get("model").and_then(|v| v.as_str()) {
                cfg.model = v.to_string();
            }
            if let Some(v) = t.get("dimensions").and_then(|v| v.as_integer()) {
                cfg.dimensions = Some(v as usize);
            }
            if let Some(v) = t.get("cache_ttl").and_then(|v| v.as_integer()) {
                cfg.cache_ttl = v as u64;
            }
        }
        Ok(cfg)
    }
}

// ── Embedding request/response ────────────────────────────────────────────────

/// Request to generate one or more embeddings.
#[derive(Debug, Clone)]
pub struct EmbeddingRequest {
    /// Text inputs to embed.
    pub texts: Vec<String>,
    /// Override model.
    pub model: Option<String>,
}

/// A single embedding result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Embedding {
    /// The embedding vector.
    pub vector: Vec<f32>,
    /// Index of the input text this corresponds to.
    pub index: usize,
}

/// Response containing one or more embeddings.
#[derive(Debug, Clone)]
pub struct EmbeddingResponse {
    pub embeddings: Vec<Embedding>,
    pub model: String,
    pub usage_tokens: usize,
}

// ── Provider trait ────────────────────────────────────────────────────────────

/// Trait for embedding providers.
pub trait EmbeddingProvider: Send + Sync {
    fn name(&self) -> &str;
    fn embed(
        &self,
        request: &EmbeddingRequest,
        config: &EmbeddingConfig,
    ) -> Result<EmbeddingResponse>;
}

// ── OpenAI Provider ───────────────────────────────────────────────────────────

/// OpenAI embedding provider (text-embedding-3-small / text-embedding-3-large).
pub struct OpenAiEmbeddingProvider;

impl EmbeddingProvider for OpenAiEmbeddingProvider {
    fn name(&self) -> &str {
        "openai"
    }

    fn embed(
        &self,
        request: &EmbeddingRequest,
        config: &EmbeddingConfig,
    ) -> Result<EmbeddingResponse> {
        let api_key = std::env::var("OPENAI_API_KEY").context("OPENAI_API_KEY not set")?;

        let model = request.model.as_deref().unwrap_or(&config.model);
        let client = reqwest::blocking::Client::new();

        let mut payload = serde_json::json!({
            "model": model,
            "input": request.texts,
        });
        if let Some(dims) = config.dimensions {
            payload["dimensions"] = serde_json::json!(dims);
        }

        let resp = client
            .post("https://api.openai.com/v1/embeddings")
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .context("OpenAI embedding request failed")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().unwrap_or_default();
            anyhow::bail!("OpenAI embedding failed: {} - {}", status, &body[..body.len().min(500)]);
        }

        let result: serde_json::Value = resp.json().context("parse embedding response")?;
        let usage_tokens = result["usage"]["total_tokens"].as_u64().unwrap_or(0) as usize;

        let data = result["data"].as_array().context("missing data array in embedding response")?;

        let embeddings: Vec<Embedding> = data
            .iter()
            .map(|item| Embedding {
                vector: item["embedding"]
                    .as_array()
                    .map(|arr| arr.iter().filter_map(|v| v.as_f64().map(|f| f as f32)).collect())
                    .unwrap_or_default(),
                index: item["index"].as_u64().unwrap_or(0) as usize,
            })
            .collect();

        Ok(EmbeddingResponse {
            embeddings,
            model: model.to_string(),
            usage_tokens,
        })
    }
}

// ── Local Provider (hash-based fallback) ──────────────────────────────────────

/// Local embedding provider using deterministic hashing (no API key needed).
/// Useful for offline mode or testing. NOT semantically meaningful.
pub struct LocalEmbeddingProvider {
    dimensions: usize,
}

impl LocalEmbeddingProvider {
    pub fn new(dimensions: usize) -> Self {
        Self { dimensions }
    }
}

impl EmbeddingProvider for LocalEmbeddingProvider {
    fn name(&self) -> &str {
        "local"
    }

    fn embed(
        &self,
        request: &EmbeddingRequest,
        _config: &EmbeddingConfig,
    ) -> Result<EmbeddingResponse> {
        let embeddings: Vec<Embedding> = request
            .texts
            .iter()
            .enumerate()
            .map(|(i, text)| {
                let vector = deterministic_embedding(text, self.dimensions);
                Embedding { vector, index: i }
            })
            .collect();

        Ok(EmbeddingResponse {
            embeddings,
            model: "local-hash".to_string(),
            usage_tokens: 0,
        })
    }
}

/// Generate a deterministic embedding vector from text using hashing.
/// NOT semantically meaningful — only for testing/caching/offline.
fn deterministic_embedding(text: &str, dimensions: usize) -> Vec<f32> {
    use std::hash::{Hash, Hasher};
    let mut hasher = rustc_hash::FxHasher::default();
    text.hash(&mut hasher);
    let seed = hasher.finish();

    // Simple deterministic vector generation.
    (0..dimensions)
        .map(|i| {
            let h = seed.wrapping_mul((i as u64).wrapping_add(1));
            // Map to [-1.0, 1.0]
            ((h as f64) / (u64::MAX as f64) * 2.0 - 1.0) as f32
        })
        .collect()
}

// ── High-level API ────────────────────────────────────────────────────────────

/// Get embeddings for text inputs, using cache when available.
/// Falls back to local provider if cloud provider fails.
pub fn get_embeddings(
    paths: &PraxisPaths,
    texts: &[String],
    model: Option<&str>,
) -> Result<Vec<Vec<f32>>> {
    let config = EmbeddingConfig::load(paths)?;
    let mut cache = EmbeddingCache::new(paths);
    let _ = cache.load(); // best-effort

    let mut results: Vec<Option<Vec<f32>>> = vec![None; texts.len()];
    let mut uncached: Vec<(usize, String)> = Vec::new();

    // Check cache first
    for (i, text) in texts.iter().enumerate() {
        let cache_key = format!("{}:{}", config.model, text);
        if let Some(entry) = cache.get(&cache_key) {
            results[i] = Some(entry.vector.clone());
        } else {
            uncached.push((i, text.clone()));
        }
    }

    if !uncached.is_empty() {
        let request = EmbeddingRequest {
            texts: uncached.iter().map(|(_, t)| t.clone()).collect(),
            model: model.map(|m| m.to_string()),
        };

        // Try primary provider, fall back to local
        let response = match config.provider.as_str() {
            "openai" => {
                let provider = OpenAiEmbeddingProvider;
                provider.embed(&request, &config)
            }
            _ => {
                let provider = LocalEmbeddingProvider::new(config.dimensions.unwrap_or(1536));
                provider.embed(&request, &config)
            }
        };

        let response = match response {
            Ok(r) => r,
            Err(e) => {
                log::warn!("embedding: primary provider failed: {e}, falling back to local");
                let local = LocalEmbeddingProvider::new(config.dimensions.unwrap_or(1536));
                local.embed(&request, &config)?
            }
        };

        // Fill results and update cache
        for emb in &response.embeddings {
            let original_index = uncached[emb.index].0;
            results[original_index] = Some(emb.vector.clone());

            let cache_key = format!("{}:{}", config.model, uncached[emb.index].1);
            cache.put(cache_key, emb.vector.clone(), config.cache_ttl);
        }

        let _ = cache.save(); // best-effort
    }

    Ok(results.into_iter().map(|o| o.unwrap_or_default()).collect())
}

/// Compute cosine similarity between two vectors.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    dot / (norm_a * norm_b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deterministic_embedding_dimensions() {
        let vec = deterministic_embedding("hello world", 128);
        assert_eq!(vec.len(), 128);
        assert!(vec.iter().all(|v| (-1.0..=1.0).contains(v)));
    }

    #[test]
    fn test_cosine_similarity_identical() {
        let vec = deterministic_embedding("test", 64);
        let sim = cosine_similarity(&vec, &vec);
        assert!((sim - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_cosine_similarity_different() {
        let a = deterministic_embedding("hello", 64);
        let b = deterministic_embedding("goodbye", 64);
        let sim = cosine_similarity(&a, &b);
        assert!(sim < 0.99); // Not identical
    }

    #[test]
    fn test_cosine_similarity_empty() {
        assert_eq!(cosine_similarity(&[], &[]), 0.0);
        assert_eq!(cosine_similarity(&[1.0], &[1.0, 2.0]), 0.0);
    }

    #[test]
    fn test_local_provider() {
        let provider = LocalEmbeddingProvider::new(64);
        let config = EmbeddingConfig::default();
        let request = EmbeddingRequest {
            texts: vec!["hello".to_string(), "world".to_string()],
            model: None,
        };
        let response = provider.embed(&request, &config).unwrap();
        assert_eq!(response.embeddings.len(), 2);
        assert_eq!(response.embeddings[0].vector.len(), 64);
        assert_eq!(response.model, "local-hash");
    }
}
