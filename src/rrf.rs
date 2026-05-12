//! Reciprocal Rank Fusion — Better hybrid search ranking.
//!
//! RRF combines vector + full-text search results.
//! This improves search quality by blending multiple ranking algorithms.

use std::collections::HashMap;

/// RRF search result with combined scoring.
#[derive(Debug, Clone)]
pub struct RrfResult {
    /// The document ID or path.
    pub id: String,
    /// Combined RRF score.
    pub score: f32,
    /// Rank from each source.
    pub ranks: HashMap<String, usize>,
}

/// Compute Reciprocal Rank Fusion score.
pub fn rrf_score(rank: usize, k: usize) -> f32 {
    1.0 / (k as f32 + rank as f32)
}

/// Combine multiple ranked result sets using RRF.
pub fn combine_rrf(results: Vec<(String, Vec<(String, usize)>)>, k: usize) -> Vec<RrfResult> {
    let mut combined: HashMap<String, (f32, HashMap<String, usize>)> = HashMap::new();

    for (source, items) in results {
        for (id, rank) in items {
            let score = rrf_score(rank, k);
            let entry = combined.entry(id.clone()).or_insert((0.0, HashMap::new()));
            entry.0 += score;
            entry.1.insert(source.clone(), rank);
        }
    }

    let mut results: Vec<RrfResult> = combined
        .into_iter()
        .map(|(id, (score, ranks))| RrfResult { id, score, ranks })
        .collect();

    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    results
}
