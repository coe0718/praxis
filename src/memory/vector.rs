//! Vector embedding helpers for hybrid semantic + keyword memory search.
//!
//! Embeddings are stored as BLOBs of little-endian f32 values in SQLite.
//! Cosine similarity is computed in Rust after loading candidate vectors.
//!
//! The current embedding generator is a deterministic hash-based placeholder
//! that produces consistent vectors from text. In production, replace
//! `generate_embedding` with a call to an OpenAI / local embedding API.

use anyhow::Result;

/// Default embedding dimension. Must match the BLOB size stored in SQLite.
pub const EMBEDDING_DIM: usize = 128;

/// Generate a deterministic placeholder embedding from text.
///
/// This is a hash-based approach for offline operation. Replace with
/// `text-embedding-3-small` or a local model for real semantic search.
pub fn generate_embedding(text: &str) -> Vec<f32> {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut vec = vec![0.0f32; EMBEDDING_DIM];
    let chunks: Vec<&str> = text.split_whitespace().collect();

    for (i, chunk) in chunks.iter().enumerate() {
        let mut hasher = DefaultHasher::new();
        chunk.hash(&mut hasher);
        let h = hasher.finish();
        for j in 0..EMBEDDING_DIM.min(8) {
            let idx = (i * 8 + j) % EMBEDDING_DIM;
            let val = ((h >> (j * 8)) & 0xFF) as f32 / 128.0 - 1.0;
            vec[idx] += val;
        }
    }

    // L2-normalise
    let norm: f32 = vec.iter().map(|v| v * v).sum::<f32>().sqrt();
    if norm > 0.0 {
        for v in &mut vec {
            *v /= norm;
        }
    }

    vec
}

/// Compute cosine similarity between two equal-length vectors.
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
    (dot / (norm_a * norm_b)).clamp(-1.0, 1.0)
}

/// Serialise a Vec<f32> into a little-endian byte blob for SQLite.
pub fn embedding_to_blob(vec: &[f32]) -> Vec<u8> {
    vec.iter().flat_map(|f| f.to_le_bytes()).collect()
}

/// Deserialise a little-endian byte blob back into Vec<f32>.
pub fn blob_to_embedding(blob: &[u8]) -> Result<Vec<f32>> {
    if !blob.len().is_multiple_of(4) {
        anyhow::bail!("embedding blob length {} is not a multiple of 4", blob.len());
    }
    let mut vec = Vec::with_capacity(blob.len() / 4);
    for chunk in blob.chunks_exact(4) {
        let bytes: [u8; 4] = chunk.try_into().expect("chunk is 4 bytes");
        vec.push(f32::from_le_bytes(bytes));
    }
    Ok(vec)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedding_is_normalised() {
        let e = generate_embedding("hello world");
        assert_eq!(e.len(), EMBEDDING_DIM);
        let norm: f32 = e.iter().map(|v| v * v).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 0.001, "embedding not normalised: {norm}");
    }

    #[test]
    fn cosine_self_similarity_is_one() {
        let e = generate_embedding("test");
        let sim = cosine_similarity(&e, &e);
        assert!((sim - 1.0).abs() < 0.001, "self-similarity should be ~1.0, got {sim}");
    }

    #[test]
    fn blob_roundtrip() {
        let e = generate_embedding("roundtrip test");
        let blob = embedding_to_blob(&e);
        let back = blob_to_embedding(&blob).unwrap();
        assert_eq!(e, back);
    }
}
