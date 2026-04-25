//! Session search — full-text search across past session outcomes and
//! action summaries. Lets operators and the agent find relevant past work.
//!
//! Backed by SQLite LIKE queries on the sessions table (outcome + action_summary
//! + selected_goal_title + selected_task columns).

use anyhow::Result;

use crate::storage::StoredSession;

/// A lightweight search result from a session lookup.
#[derive(Debug, Clone)]
pub struct SessionSearchResult {
    pub session: StoredSession,
    /// Which column matched the query.
    pub matched_column: String,
    /// A snippet of the matching text (up to 200 chars).
    pub snippet: String,
}

/// Extension trait for session stores that support search.
pub trait SessionSearchStore {
    /// Search sessions by keyword match on outcome, action summary, goal title,
    /// and selected task. Returns results ordered by most recent first.
    fn search_sessions(&self, query: &str, limit: usize) -> Result<Vec<SessionSearchResult>>;
}
