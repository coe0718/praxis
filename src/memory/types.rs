use chrono::{DateTime, Utc};

use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryTier {
    Hot,
    Cold,
}

/// Semantic class of a memory — governs decay rate and retrieval priority.
///
/// | Type        | Decay threshold | Example content                       |
/// |-------------|----------------|---------------------------------------|
/// | Episodic    | 90 days        | "Operator approved X on Tuesday"      |
/// | Semantic    | 180 days       | "Operator prefers concise replies"    |
/// | Procedural  | 365 days       | "Always run tests before committing"  |
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryType {
    /// Time-stamped autobiographical events — fastest decay.
    #[default]
    Episodic,
    /// Generalised facts and preferences — medium decay.
    Semantic,
    /// How-to knowledge and workflow rules — slowest decay.
    Procedural,
}

impl MemoryType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Episodic => "episodic",
            Self::Semantic => "semantic",
            Self::Procedural => "procedural",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s {
            "semantic" => Self::Semantic,
            "procedural" => Self::Procedural,
            _ => Self::Episodic,
        }
    }

    /// Days a cold memory of this type survives without reinforcement.
    pub fn decay_days(self) -> i64 {
        match self {
            Self::Episodic => 90,
            Self::Semantic => 180,
            Self::Procedural => 365,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NewHotMemory {
    pub content: String,
    pub summary: Option<String>,
    pub importance: f32,
    pub tags: Vec<String>,
    pub expires_at: Option<String>,
    #[serde(default)]
    pub memory_type: MemoryType,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NewColdMemory {
    pub content: String,
    pub weight: f32,
    pub tags: Vec<String>,
    pub source_ids: Vec<i64>,
    pub contradicts: Vec<i64>,
    #[serde(default)]
    pub memory_type: MemoryType,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StoredMemory {
    pub id: i64,
    pub tier: MemoryTier,
    pub content: String,
    pub summary: Option<String>,
    pub tags: Vec<String>,
    pub score: f32,
    #[serde(default)]
    pub memory_type: MemoryType,
}

// ── Memory links ──────────────────────────────────────────────────────────────

/// The typed relationship between two memories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryLinkType {
    /// This memory was a direct cause of the linked memory.
    CausedBy,
    /// Two memories are topically related.
    RelatedTo,
    /// This memory conflicts with the linked memory.
    Contradicts,
    /// This memory records an operator preference.
    UserPreference,
    /// This memory is a follow-up to the linked memory.
    FollowUp,
}

impl MemoryLinkType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::CausedBy => "caused_by",
            Self::RelatedTo => "related_to",
            Self::Contradicts => "contradicts",
            Self::UserPreference => "user_preference",
            Self::FollowUp => "follow_up",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "caused_by" => Some(Self::CausedBy),
            "related_to" => Some(Self::RelatedTo),
            "contradicts" => Some(Self::Contradicts),
            "user_preference" => Some(Self::UserPreference),
            "follow_up" => Some(Self::FollowUp),
            _ => None,
        }
    }
}

impl std::fmt::Display for MemoryLinkType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryLink {
    pub id: i64,
    pub from_memory_id: i64,
    pub to_memory_id: i64,
    pub link_type: MemoryLinkType,
}

// ── Traits ────────────────────────────────────────────────────────────────────

pub struct ConsolidationSummary {
    pub consolidated: usize,
    pub pruned: usize,
}

pub trait MemoryStore {
    fn insert_hot_memory(&self, memory: NewHotMemory) -> Result<StoredMemory>;
    fn insert_cold_memory(&self, memory: NewColdMemory) -> Result<StoredMemory>;
    fn recent_hot_memories(&self, limit: usize) -> Result<Vec<StoredMemory>>;
    fn strongest_cold_memories(&self, limit: usize) -> Result<Vec<StoredMemory>>;
    fn search_memories(&self, query: &str, limit: usize) -> Result<Vec<StoredMemory>>;

    /// Semantic search using stored embeddings + cosine similarity.
    /// Falls back to FTS5 if no embeddings are present.
    fn search_memories_semantic(&self, query: &str, limit: usize) -> Result<Vec<StoredMemory>>;

    fn decay_cold_memories(&self, now: DateTime<Utc>) -> Result<usize>;

    /// Cluster related hot memories into cold memories and prune dead cold memories.
    /// Returns counts of clusters promoted and cold memories pruned.
    fn consolidate_memories(&self, now: DateTime<Utc>) -> Result<ConsolidationSummary>;

    /// Fetch a single memory by ID, checking both tiers.
    fn get_memory(&self, id: i64) -> Result<Option<StoredMemory>>;

    /// Increase the importance/weight of a memory by a fixed step.
    /// Returns `true` if the memory was found and boosted.
    fn boost_memory(&self, id: i64) -> Result<bool>;

    /// Immediately remove a memory from the store (hard delete).
    /// Returns `true` if the memory was found and deleted.
    fn forget_memory(&self, id: i64) -> Result<bool>;
}

/// Typed relational links between memories.
pub trait MemoryLinkStore {
    /// Record a directed link between two memories.  Idempotent.
    fn add_memory_link(&self, from_id: i64, to_id: i64, link_type: MemoryLinkType) -> Result<()>;

    /// All links involving `memory_id` as either the source or target.
    fn links_for(&self, memory_id: i64) -> Result<Vec<MemoryLink>>;

    /// Memories reachable via any link from `memory_id`, up to `limit`.
    fn linked_memories(&self, memory_id: i64, limit: usize) -> Result<Vec<StoredMemory>>;
}
