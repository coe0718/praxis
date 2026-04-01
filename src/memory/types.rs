use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryTier {
    Hot,
    Cold,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NewHotMemory {
    pub content: String,
    pub summary: Option<String>,
    pub importance: f32,
    pub tags: Vec<String>,
    pub expires_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NewColdMemory {
    pub content: String,
    pub weight: f32,
    pub tags: Vec<String>,
    pub source_ids: Vec<i64>,
    pub contradicts: Vec<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StoredMemory {
    pub id: i64,
    pub tier: MemoryTier,
    pub content: String,
    pub summary: Option<String>,
    pub tags: Vec<String>,
    pub score: f32,
}

pub trait MemoryStore {
    fn insert_hot_memory(&self, memory: NewHotMemory) -> Result<StoredMemory>;
    fn insert_cold_memory(&self, memory: NewColdMemory) -> Result<StoredMemory>;
    fn recent_hot_memories(&self, limit: usize) -> Result<Vec<StoredMemory>>;
    fn strongest_cold_memories(&self, limit: usize) -> Result<Vec<StoredMemory>>;
    fn search_memories(&self, query: &str, limit: usize) -> Result<Vec<StoredMemory>>;
}
