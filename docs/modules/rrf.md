# Reciprocal Rank Fusion (RRF)

> Better hybrid search ranking by combining vector and full-text search results.

## Overview

The RRF module provides a minimal implementation of Reciprocal Rank Fusion for combining multiple ranked result sets into a single, consensus-driven ranking. This improves search quality by blending scores from different retrieval algorithms (e.g., vector similarity and BM25 full-text search).

The formula is straightforward: `rrf_score(rank, k) = 1.0 / (k + rank)`, where `k` is a fusion constant (typically 60). Results from each source are scored, summed per document, and sorted descending.

## Architecture

### Key Types

| Type | Description |
|------|-------------|
| `RrfResult` | A combined search result with document ID, total RRF score, and per-source ranks. |

### Formula

```
rrf_score(rank, k) = 1 / (k + rank)

combined_score(id) = Σ rrf_score(source_rank, k) for all sources
```

## Public API

### `RrfResult`

```rust
pub struct RrfResult {
    pub id: String,
    pub score: f32,
    pub ranks: HashMap<String, usize>,
}
```

| Field | Type | Description |
|-------|------|-------------|
| `id` | `String` | Document ID or path. |
| `score` | `f32` | Combined RRF score (sum of per-source scores). |
| `ranks` | `HashMap<String, usize>` | Original rank from each source, keyed by source name. |

### Free Functions

```rust
pub fn rrf_score(rank: usize, k: usize) -> f32

pub fn combine_rrf(
    results: Vec<(String, Vec<(String, usize)>)>,
    k: usize,
) -> Vec<RrfResult>
```

- **`rrf_score`** — Computes the reciprocal rank fusion score for a single rank: `1.0 / (k + rank)`.
- **`combine_rrf`** — Takes a vector of `(source_name, [(doc_id, rank), ...])` tuples, computes combined RRF scores for all documents, and returns results sorted by score descending.

Each input entry represents a ranked result list from one search source. The function aggregates scores for documents that appear in multiple result sets.

## Configuration

No configuration fields, environment variables, or feature flags. The `k` constant is passed directly to `combine_rrf`.

## Usage

```rust
use praxis::rrf::{combine_rrf, RrfResult};

let results = vec![
    (
        "vector".to_string(),
        vec![
            ("doc_a".to_string(), 1),
            ("doc_b".to_string(), 2),
            ("doc_c".to_string(), 3),
        ],
    ),
    (
        "bm25".to_string(),
        vec![
            ("doc_b".to_string(), 1),
            ("doc_a".to_string(), 2),
            ("doc_d".to_string(), 3),
        ],
    ),
];

let combined = combine_rrf(results, 60);
for r in &combined {
    println!("{} — score: {:.4}", r.id, r.score);
}
// doc_a appears at rank 1 (vector) and rank 2 (bm25) → highest combined score
// doc_b appears at rank 2 (vector) and rank 1 (bm25) → second highest
// doc_c and doc_d appear in only one result set
```

## Data Files

None. RRF operates purely in memory.

## Dependencies

- **`std::collections::HashMap`** — Per-source rank tracking and score aggregation.

## Source

`src/rrf.rs`