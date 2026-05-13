mod anatomy;
mod approvals;
mod decisions;
mod learning;
mod memory;
mod memory_consolidation;
mod memory_decay;
mod memory_links;
mod opportunities;
mod ops;
mod providers;
mod quality;
mod schema;
mod schema_data;
mod search;
mod sessions;

pub(crate) use memory_links::ContradictionQuery;

#[cfg(test)]
mod insights_tests;

#[cfg(test)]
mod tests;

use std::{fs, path::PathBuf, sync::Arc};

use anyhow::{Context, Result};
use rusqlite::Connection;

use crate::storage::{
    ApprovalStatus, ApprovalStore, NewApprovalRequest, SessionRecord, SessionStore,
    StoredApprovalRequest, StoredSession,
};

#[derive(Debug, Clone)]
pub struct SqliteSessionStore {
    path: PathBuf,
    // S7 fix: Connection pool for concurrent database access
    pool: Option<Arc<parking_lot::Mutex<Vec<Connection>>>>,
    pool_max_size: usize,
}

impl SqliteSessionStore {
    pub fn new(path: PathBuf) -> Self {
        Self { 
            path, 
            pool: None,
            pool_max_size: 5,
        }
    }
    
    /// S7 fix: Initialize connection pool for concurrent database access
    pub fn with_pool(path: PathBuf, max_connections: usize) -> Result<Self> {
        let mut pool = Vec::new();
        for _ in 0..max_connections {
            let conn = Self::create_connection(&path)?;
            pool.push(conn);
        }
        Ok(Self { 
            path, 
            pool: Some(Arc::new(parking_lot::Mutex::new(pool))),
            pool_max_size: max_connections,
        })
    }
    
    /// S7 fix: Get a pooled connection, or create a new one if pool not initialized
    pub fn get_connection(&self) -> Result<Connection> {
        if let Some(ref pool) = self.pool {
            let mut pool_guard = pool.lock();
            // Take a connection from the pool, or create a new one if empty
            if let Some(conn) = pool_guard.pop() {
                Ok(conn)
            } else {
                Self::create_connection(&self.path)
            }
        } else {
            Self::create_connection(&self.path)
        }
    }
    
    /// S7 fix: Return a connection to the pool
    pub fn return_connection(&self, conn: Connection) {
        if let Some(ref pool) = self.pool {
            let mut pool_guard = pool.lock();
            if pool_guard.len() < self.pool_max_size {
                pool_guard.push(conn);
            }
            // If pool is full, just drop the connection
        }
    }
    
    fn create_connection(path: &PathBuf) -> Result<Connection> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }

        let conn = Connection::open(path)
            .with_context(|| format!("failed to open SQLite database {}", path.display()))?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA busy_timeout=5000;")
            .context("failed to configure SQLite WAL mode")?;
        Ok(conn)
    }

    pub fn search_approvals(
        &self,
        q: Option<&str>,
        tool: Option<&str>,
        status: Option<ApprovalStatus>,
    ) -> Result<Vec<StoredApprovalRequest>> {
        approvals::search_approvals(self, q, tool, status)
    }

    pub fn token_summary_all_time(&self) -> Result<crate::usage::TokenSummaryAllTime> {
        providers::token_summary_all_time(self)
    }

    pub fn token_usage_by_session(
        &self,
        limit: usize,
    ) -> Result<Vec<crate::usage::SessionTokenUsage>> {
        providers::token_usage_by_session(self, limit)
    }

    pub fn token_usage_by_provider(&self) -> Result<Vec<crate::usage::ProviderTokenSummary>> {
        providers::token_usage_by_provider(self)
    }

    pub fn count_hot_memories(&self) -> Result<i64> {
        let conn = self.get_connection()?;
        let result = conn.query_row("SELECT COUNT(*) FROM hot_memories", [], |row| row.get(0))
            .context("failed to count hot memories")?;
        self.return_connection(conn);
        Ok(result)
    }

    pub fn count_cold_memories(&self) -> Result<i64> {
        let conn = self.get_connection()?;
        let result = conn.query_row("SELECT COUNT(*) FROM cold_memories", [], |row| row.get(0))
            .context("failed to count cold memories")?;
        self.return_connection(conn);
        Ok(result)
    }

    pub fn count_pending_approvals(&self) -> Result<i64> {
        let conn = self.get_connection()?;
        let result = conn
            .query_row(
                "SELECT COUNT(*) FROM approval_requests WHERE status = 'pending'",
                [],
                |row| row.get(0),
            )
            .context("failed to count pending approvals")?;
        self.return_connection(conn);
        Ok(result)
    }

    /// Batch COUNT queries on a single connection for health checks.
    pub fn health_counts(&self) -> Result<(i64, i64, i64)> {
        let conn = self.get_connection()?;
        let pending = conn
            .query_row(
                "SELECT COUNT(*) FROM approval_requests WHERE status = 'pending'",
                [],
                |row| row.get(0),
            )
            .context("failed to count pending approvals")?;
        let hot = conn
            .query_row("SELECT COUNT(*) FROM hot_memories", [], |row| row.get(0))
            .context("failed to count hot memories")?;
        let cold = conn
            .query_row("SELECT COUNT(*) FROM cold_memories", [], |row| row.get(0))
            .context("failed to count cold memories")?;
        self.return_connection(conn);
        Ok((pending, hot, cold))
    }

    pub fn search_sessions(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<crate::storage::SessionSearchResult>> {
        use crate::storage::search::SessionSearchStore;
        SessionSearchStore::search_sessions(self, query, limit)
    }
}

impl SessionStore for SqliteSessionStore {
    fn initialize(&self) -> Result<()> {
        schema::initialize(self)
    }

    fn validate_schema(&self) -> Result<()> {
        schema::validate(self)
    }

    fn record_session(&self, record: &SessionRecord) -> Result<StoredSession> {
        sessions::record_session(self, record)
    }

    fn last_session(&self) -> Result<Option<StoredSession>> {
        sessions::last_session(self)
    }
}

impl ApprovalStore for SqliteSessionStore {
    fn queue_approval(&self, request: &NewApprovalRequest) -> Result<StoredApprovalRequest> {
        approvals::queue_approval(self, request)
    }

    fn list_approvals(&self, status: Option<ApprovalStatus>) -> Result<Vec<StoredApprovalRequest>> {
        approvals::list_approvals(self, status)
    }

    fn get_approval(&self, id: i64) -> Result<Option<StoredApprovalRequest>> {
        approvals::get_approval(self, id)
    }

    fn set_approval_status(
        &self,
        id: i64,
        status: ApprovalStatus,
        note: Option<&str>,
    ) -> Result<Option<StoredApprovalRequest>> {
        approvals::set_approval_status(self, id, status, note)
    }

    fn next_approved_request(&self) -> Result<Option<StoredApprovalRequest>> {
        approvals::next_approved_request(self)
    }

    fn mark_approval_consumed(&self, id: i64) -> Result<()> {
        approvals::mark_approval_consumed(self, id)
    }
}