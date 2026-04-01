use anyhow::{Context, Result};
use rusqlite::{OptionalExtension, params};

use crate::storage::{
    EvalRunRecord, QualityStore, ReviewRecord, ReviewStatus, SessionQualityUpdate,
    StoredEvalSummary, StoredReview,
};

use super::SqliteSessionStore;

pub(super) fn update_session_quality(
    store: &SqliteSessionStore,
    session_id: i64,
    update: &SessionQualityUpdate,
) -> Result<()> {
    let connection = store.connect()?;
    connection
        .execute(
            "
            UPDATE sessions
            SET outcome = ?2,
                action_summary = ?3,
                reviewer_passes = ?4,
                reviewer_failures = ?5,
                eval_passes = ?6,
                eval_failures = ?7
            WHERE id = ?1
            ",
            params![
                session_id,
                update.outcome,
                update.action_summary,
                update.reviewer_passes,
                update.reviewer_failures,
                update.eval_passes,
                update.eval_failures,
            ],
        )
        .context("failed to update session quality metadata")?;
    Ok(())
}

pub(super) fn record_review(
    store: &SqliteSessionStore,
    record: &ReviewRecord,
) -> Result<StoredReview> {
    let connection = store.connect()?;
    let reviewed_at = record.reviewed_at.to_rfc3339();
    connection
        .execute(
            "
            INSERT INTO review_runs(session_id, goal_id, status, summary, findings_json, reviewed_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            ",
            params![
                record.session_id,
                record.goal_id,
                record.status.as_str(),
                record.summary,
                record.findings_json,
                reviewed_at,
            ],
        )
        .context("failed to insert review row")?;

    Ok(StoredReview {
        id: connection.last_insert_rowid(),
        session_id: record.session_id,
        goal_id: record.goal_id.clone(),
        status: record.status,
        summary: record.summary.clone(),
        findings_json: record.findings_json.clone(),
        reviewed_at,
    })
}

pub(super) fn last_review(store: &SqliteSessionStore) -> Result<Option<StoredReview>> {
    let connection = store.connect()?;
    connection
        .query_row(
            "
            SELECT id, session_id, goal_id, status, summary, findings_json, reviewed_at
            FROM review_runs
            ORDER BY id DESC
            LIMIT 1
            ",
            [],
            |row| {
                let status = ReviewStatus::parse(&row.get::<_, String>(3)?).map_err(|error| {
                    rusqlite::Error::FromSqlConversionFailure(
                        3,
                        rusqlite::types::Type::Text,
                        Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, error)),
                    )
                })?;
                Ok(StoredReview {
                    id: row.get(0)?,
                    session_id: row.get(1)?,
                    goal_id: row.get(2)?,
                    status,
                    summary: row.get(4)?,
                    findings_json: row.get(5)?,
                    reviewed_at: row.get(6)?,
                })
            },
        )
        .optional()
        .context("failed to load the most recent review")
}

pub(super) fn record_eval_run(store: &SqliteSessionStore, record: &EvalRunRecord) -> Result<()> {
    let connection = store.connect()?;
    connection
        .execute(
            "
            INSERT INTO eval_runs(session_id, eval_id, eval_name, status, severity, summary, evaluated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            ",
            params![
                record.session_id,
                record.eval_id,
                record.eval_name,
                record.status.as_str(),
                record.severity.as_str(),
                record.summary,
                record.evaluated_at.to_rfc3339(),
            ],
        )
        .context("failed to insert eval row")?;
    Ok(())
}

pub(super) fn latest_eval_summary(store: &SqliteSessionStore) -> Result<Option<StoredEvalSummary>> {
    let connection = store.connect()?;
    connection
        .query_row(
            "
            SELECT session_id,
                   SUM(CASE WHEN status = 'passed' THEN 1 ELSE 0 END) AS passed,
                   SUM(CASE WHEN status = 'failed' THEN 1 ELSE 0 END) AS failed,
                   SUM(CASE WHEN status = 'skipped' THEN 1 ELSE 0 END) AS skipped,
                   SUM(CASE WHEN status = 'failed' AND severity = 'trust_damaging' THEN 1 ELSE 0 END) AS trust_failures
            FROM eval_runs
            WHERE session_id = (SELECT session_id FROM eval_runs ORDER BY id DESC LIMIT 1)
            GROUP BY session_id
            ",
            [],
            |row| {
                Ok(StoredEvalSummary {
                    session_id: row.get(0)?,
                    passed: row.get(1)?,
                    failed: row.get(2)?,
                    skipped: row.get(3)?,
                    trust_failures: row.get(4)?,
                })
            },
        )
        .optional()
        .context("failed to load the latest eval summary")
}

impl QualityStore for SqliteSessionStore {
    fn update_session_quality(&self, session_id: i64, update: &SessionQualityUpdate) -> Result<()> {
        update_session_quality(self, session_id, update)
    }

    fn record_review(&self, record: &ReviewRecord) -> Result<StoredReview> {
        record_review(self, record)
    }

    fn last_review(&self) -> Result<Option<StoredReview>> {
        last_review(self)
    }

    fn record_eval_run(&self, record: &EvalRunRecord) -> Result<()> {
        record_eval_run(self, record)
    }

    fn latest_eval_summary(&self) -> Result<Option<StoredEvalSummary>> {
        latest_eval_summary(self)
    }
}
