mod approvals;
mod memory;
mod schema;
mod sessions;

#[cfg(test)]
mod tests;

use std::{fs, path::PathBuf};

use anyhow::{Context, Result};
use rusqlite::Connection;

use crate::storage::{
    ApprovalStatus, ApprovalStore, NewApprovalRequest, SessionRecord, SessionStore,
    StoredApprovalRequest, StoredSession,
};

#[derive(Debug, Clone)]
pub struct SqliteSessionStore {
    path: PathBuf,
}

impl SqliteSessionStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    fn connect(&self) -> Result<Connection> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }

        Connection::open(&self.path)
            .with_context(|| format!("failed to open SQLite database {}", self.path.display()))
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
