use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{OptionalExtension, params};

use crate::learning::{OpportunityCandidate, OpportunityStatus, StoredOpportunity};

use super::SqliteSessionStore;

impl SqliteSessionStore {
    pub fn create_opportunity(
        &self,
        candidate: &OpportunityCandidate,
        now: DateTime<Utc>,
    ) -> Result<StoredOpportunity> {
        let connection = self.connect()?;
        let created_at = now.to_rfc3339();
        connection
            .execute(
                "
                INSERT INTO opportunities(
                    signature, kind, title, summary, evidence_json, status, goal_id, created_at, updated_at
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, NULL, ?7, ?7)
                ",
                params![
                    candidate.signature,
                    candidate.kind,
                    candidate.title,
                    candidate.summary,
                    candidate.evidence_json,
                    OpportunityStatus::Pending.as_str(),
                    created_at,
                ],
            )
            .context("failed to create opportunity")?;
        Ok(StoredOpportunity {
            id: connection.last_insert_rowid(),
            signature: candidate.signature.clone(),
            kind: candidate.kind.clone(),
            title: candidate.title.clone(),
            summary: candidate.summary.clone(),
            status: OpportunityStatus::Pending.as_str().to_string(),
            goal_id: None,
            created_at,
            updated_at: now.to_rfc3339(),
        })
    }

    pub fn list_opportunities(
        &self,
        status: OpportunityStatus,
        limit: usize,
    ) -> Result<Vec<StoredOpportunity>> {
        let connection = self.connect()?;
        let mut statement = connection
            .prepare(
                "
                SELECT id, signature, kind, title, summary, status, goal_id, created_at, updated_at
                FROM opportunities
                WHERE status = ?1
                ORDER BY id DESC
                LIMIT ?2
                ",
            )
            .context("failed to prepare opportunity query")?;
        let rows = statement
            .query_map(params![status.as_str(), limit as i64], |row| {
                Ok(StoredOpportunity {
                    id: row.get(0)?,
                    signature: row.get(1)?,
                    kind: row.get(2)?,
                    title: row.get(3)?,
                    summary: row.get(4)?,
                    status: row.get(5)?,
                    goal_id: row.get(6)?,
                    created_at: row.get(7)?,
                    updated_at: row.get(8)?,
                })
            })
            .context("failed to execute opportunity query")?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .context("failed to load opportunities")
    }

    pub fn pending_opportunity_count(&self) -> Result<i64> {
        let connection = self.connect()?;
        connection
            .query_row(
                "SELECT COUNT(*) FROM opportunities WHERE status = ?1",
                params![OpportunityStatus::Pending.as_str()],
                |row| row.get(0),
            )
            .context("failed to count pending opportunities")
    }

    pub fn has_opportunity_signature(&self, signature: &str) -> Result<bool> {
        let connection = self.connect()?;
        let found: Option<i64> = connection
            .query_row(
                "SELECT id FROM opportunities WHERE signature = ?1 LIMIT 1",
                params![signature],
                |row| row.get(0),
            )
            .optional()
            .context("failed to query opportunity signature")?;
        Ok(found.is_some())
    }

    pub fn count_opportunities_since(&self, since: DateTime<Utc>) -> Result<i64> {
        let connection = self.connect()?;
        connection
            .query_row(
                "SELECT COUNT(*) FROM opportunities WHERE created_at >= ?1",
                params![since.to_rfc3339()],
                |row| row.get(0),
            )
            .context("failed to count recent opportunities")
    }

    pub fn set_opportunity_status(
        &self,
        id: i64,
        status: OpportunityStatus,
        now: DateTime<Utc>,
    ) -> Result<Option<StoredOpportunity>> {
        let connection = self.connect()?;
        let changed = connection
            .execute(
                "
                UPDATE opportunities
                SET status = ?1, updated_at = ?2
                WHERE id = ?3
                ",
                params![status.as_str(), now.to_rfc3339(), id],
            )
            .context("failed to update opportunity status")?;
        if changed == 0 {
            return Ok(None);
        }

        connection
            .query_row(
                "
                SELECT id, signature, kind, title, summary, status, goal_id, created_at, updated_at
                FROM opportunities
                WHERE id = ?1
                ",
                params![id],
                |row| {
                    Ok(StoredOpportunity {
                        id: row.get(0)?,
                        signature: row.get(1)?,
                        kind: row.get(2)?,
                        title: row.get(3)?,
                        summary: row.get(4)?,
                        status: row.get(5)?,
                        goal_id: row.get(6)?,
                        created_at: row.get(7)?,
                        updated_at: row.get(8)?,
                    })
                },
            )
            .optional()
            .context("failed to load updated opportunity")
    }

    pub fn get_opportunity(&self, id: i64) -> Result<Option<StoredOpportunity>> {
        let connection = self.connect()?;
        connection
            .query_row(
                "
                SELECT id, signature, kind, title, summary, status, goal_id, created_at, updated_at
                FROM opportunities
                WHERE id = ?1
                ",
                params![id],
                |row| {
                    Ok(StoredOpportunity {
                        id: row.get(0)?,
                        signature: row.get(1)?,
                        kind: row.get(2)?,
                        title: row.get(3)?,
                        summary: row.get(4)?,
                        status: row.get(5)?,
                        goal_id: row.get(6)?,
                        created_at: row.get(7)?,
                        updated_at: row.get(8)?,
                    })
                },
            )
            .optional()
            .context("failed to load opportunity")
    }

    pub fn set_opportunity_goal(
        &self,
        id: i64,
        goal_id: Option<&str>,
        now: DateTime<Utc>,
    ) -> Result<()> {
        let connection = self.connect()?;
        connection
            .execute(
                "
                UPDATE opportunities
                SET goal_id = ?1, updated_at = ?2
                WHERE id = ?3
                ",
                params![goal_id, now.to_rfc3339(), id],
            )
            .context("failed to link opportunity goal")?;
        Ok(())
    }
}
