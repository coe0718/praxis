use anyhow::Result;

use crate::memory::{NewDoNotRepeat, NewKnownBug, StoredDoNotRepeat, StoredKnownBug};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationalMemoryCounts {
    pub do_not_repeat: i64,
    pub known_bugs: i64,
}

pub trait OperationalMemoryStore {
    fn record_do_not_repeat(&self, entry: NewDoNotRepeat) -> Result<StoredDoNotRepeat>;
    fn recent_do_not_repeat(&self, limit: usize) -> Result<Vec<StoredDoNotRepeat>>;
    fn search_do_not_repeat(&self, query: &str, limit: usize) -> Result<Vec<StoredDoNotRepeat>>;
    fn record_known_bug(&self, entry: NewKnownBug) -> Result<StoredKnownBug>;
    fn recent_known_bugs(&self, limit: usize) -> Result<Vec<StoredKnownBug>>;
    fn search_known_bugs(&self, query: &str, limit: usize) -> Result<Vec<StoredKnownBug>>;
    fn operational_memory_counts(&self) -> Result<OperationalMemoryCounts>;
}
