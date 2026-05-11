//! Semantic search — find similar items using vector embeddings.
//!
//! Builds an in-memory index of embeddings for fast cosine similarity search.
//! Used by memory, skills, and knowledge retrieval.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::embedding::cosine_similarity;

/// A document in the search index.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchDocument {
    /// Unique ID.
    pub id: String,
    /// Text content (for display).
    pub text: String,
    /// Embedding vector.
    pub vector: Vec<f32>,
    /// Optional metadata.
    pub metadata: HashMap<String, String>,
    /// Timestamp when indexed.
    pub indexed_at: chrono::DateTime<chrono::Utc>,
}

/// Search result with relevance score.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// The matched document.
    pub doc: SearchDocument,
    /// Cosine similarity score (0.0 to 1.0).
    pub score: f32,
}

/// In-memory semantic search index.
#[derive(Debug, Clone, Default)]
pub struct SemanticIndex {
    documents: HashMap<String, SearchDocument>,
}

impl SemanticIndex {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a document to the index.
    pub fn add(&mut self, doc: SearchDocument) {
        self.documents.insert(doc.id.clone(), doc);
    }

    /// Remove a document by ID.
    pub fn remove(&mut self, id: &str) -> bool {
        self.documents.remove(id).is_some()
    }

    /// Get a document by ID.
    pub fn get(&self, id: &str) -> Option<&SearchDocument> {
        self.documents.get(id)
    }

    /// Number of documents in index.
    pub fn len(&self) -> usize {
        self.documents.len()
    }

    /// Whether the index is empty.
    pub fn is_empty(&self) -> bool {
        self.documents.is_empty()
    }

    /// Search for documents similar to the given vector.
    /// Returns results sorted by similarity (highest first).
    pub fn search(&self, query: &[f32], limit: usize) -> Vec<SearchResult> {
        let mut results: Vec<SearchResult> = self
            .documents
            .values()
            .map(|doc| {
                let score = cosine_similarity(query, &doc.vector);
                SearchResult { doc: doc.clone(), score }
            })
            .filter(|r| r.score > 0.0)
            .collect();

        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(limit);
        results
    }

    /// Search for documents similar to a given document ID.
    pub fn search_similar(&self, doc_id: &str, limit: usize) -> Vec<SearchResult> {
        let doc = match self.documents.get(doc_id) {
            Some(d) => d,
            None => return Vec::new(),
        };
        let query = doc.vector.clone();
        let mut results: Vec<SearchResult> = self
            .documents
            .values()
            .filter(|d| d.id != doc_id)
            .map(|d| {
                let score = cosine_similarity(&query, &d.vector);
                SearchResult { doc: d.clone(), score }
            })
            .filter(|r| r.score > 0.0)
            .collect();

        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(limit);
        results
    }

    /// Filter documents by metadata key-value pair.
    pub fn filter_by_metadata(&self, key: &str, value: &str) -> Vec<&SearchDocument> {
        self.documents
            .values()
            .filter(|d| d.metadata.get(key).map(|v| v.as_str()) == Some(value))
            .collect()
    }

    /// Get all document IDs.
    pub fn ids(&self) -> Vec<String> {
        self.documents.keys().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_doc(id: &str, vector: Vec<f32>) -> SearchDocument {
        SearchDocument {
            id: id.to_string(),
            text: format!("Document {}", id),
            vector,
            metadata: HashMap::new(),
            indexed_at: chrono::Utc::now(),
        }
    }

    fn make_doc_with_meta(
        id: &str,
        vector: Vec<f32>,
        meta: HashMap<String, String>,
    ) -> SearchDocument {
        SearchDocument {
            id: id.to_string(),
            text: format!("Document {}", id),
            vector,
            metadata: meta,
            indexed_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn test_add_and_get() {
        let mut index = SemanticIndex::new();
        let doc = make_doc("1", vec![1.0, 0.0, 0.0]);
        index.add(doc);

        assert_eq!(index.len(), 1);
        assert!(index.get("1").is_some());
        assert!(index.get("2").is_none());
    }

    #[test]
    fn test_remove() {
        let mut index = SemanticIndex::new();
        index.add(make_doc("1", vec![1.0, 0.0]));
        assert!(index.remove("1"));
        assert!(index.is_empty());
        assert!(!index.remove("nonexistent"));
    }

    #[test]
    fn test_search_by_vector() {
        let mut index = SemanticIndex::new();
        // Three docs pointing in different directions
        index.add(make_doc("a", vec![1.0, 0.0, 0.0]));
        index.add(make_doc("b", vec![0.0, 1.0, 0.0]));
        index.add(make_doc("c", vec![0.9, 0.1, 0.0])); // Similar to 'a'

        let results = index.search(&[1.0, 0.0, 0.0], 2);
        assert_eq!(results.len(), 2);
        // 'a' should be first (exact match), 'c' second (close)
        assert_eq!(results[0].doc.id, "a");
        assert!(results[0].score > 0.99);
        assert_eq!(results[1].doc.id, "c");
    }

    #[test]
    fn test_search_similar() {
        let mut index = SemanticIndex::new();
        index.add(make_doc("a", vec![1.0, 0.0, 0.0]));
        index.add(make_doc("b", vec![0.0, 1.0, 0.0]));
        index.add(make_doc("c", vec![0.9, 0.1, 0.0]));

        let results = index.search_similar("a", 2);
        assert!(results.iter().all(|r| r.doc.id != "a")); // Should not include self
        assert_eq!(results[0].doc.id, "c"); // c is most similar to a
    }

    #[test]
    fn test_filter_by_metadata() {
        let mut index = SemanticIndex::new();
        let mut meta1 = HashMap::new();
        meta1.insert("category".to_string(), "rust".to_string());
        index.add(make_doc_with_meta("1", vec![1.0], meta1));

        let mut meta2 = HashMap::new();
        meta2.insert("category".to_string(), "python".to_string());
        index.add(make_doc_with_meta("2", vec![0.0], meta2));

        let rust_docs = index.filter_by_metadata("category", "rust");
        assert_eq!(rust_docs.len(), 1);
        assert_eq!(rust_docs[0].id, "1");
    }

    #[test]
    fn test_search_empty_index() {
        let index = SemanticIndex::new();
        let results = index.search(&[1.0, 0.0], 5);
        assert!(results.is_empty());
    }

    #[test]
    fn test_search_similar_nonexistent() {
        let index = SemanticIndex::new();
        let results = index.search_similar("nonexistent", 5);
        assert!(results.is_empty());
    }
}
