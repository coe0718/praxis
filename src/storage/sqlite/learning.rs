use anyhow::{Context, Result};
use rusqlite::{OptionalExtension, params};

use crate::learning::{
    NewLearningRun, NewLearningSourceState, StoredLearningRun, StoredLearningSource,
};

use super::SqliteSessionStore;

impl SqliteSessionStore {
    pub fn list_learning_sources(&self) -> Result<Vec<StoredLearningSource>> {
        let connection = self.connect()?;
        let mut statement = connection
            .prepare(
                "
                SELECT path, last_modified_at, byte_len, summary, last_processed_at
                FROM learning_sources
                ORDER BY path
                ",
            )
            .context("failed to prepare learning source query")?;
        let rows = statement
            .query_map([], |row| {
                Ok(StoredLearningSource {
                    path: row.get(0)?,
                    last_modified_at: row.get(1)?,
                    byte_len: row.get(2)?,
                    summary: row.get(3)?,
                    last_processed_at: row.get(4)?,
                })
            })
            .context("failed to execute learning source query")?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .context("failed to load learning sources")
    }

    pub fn upsert_learning_source(&self, state: NewLearningSourceState) -> Result<()> {
        let connection = self.connect()?;
        connection
            .execute(
                "
                INSERT INTO learning_sources(path, last_modified_at, byte_len, summary, last_processed_at)
                VALUES (?1, ?2, ?3, ?4, ?5)
                ON CONFLICT(path) DO UPDATE SET
                    last_modified_at = excluded.last_modified_at,
                    byte_len = excluded.byte_len,
                    summary = excluded.summary,
                    last_processed_at = excluded.last_processed_at
                ",
                params![
                    state.path,
                    state.last_modified_at,
                    state.byte_len,
                    state.summary,
                    state.last_processed_at.to_rfc3339(),
                ],
            )
            .context("failed to upsert learning source state")?;
        Ok(())
    }

    pub fn record_learning_run(&self, run: NewLearningRun) -> Result<StoredLearningRun> {
        let connection = self.connect()?;
        let notes_json =
            serde_json::to_string(&run.notes).context("failed to serialize learning notes")?;
        let completed_at = run.completed_at.to_rfc3339();
        connection
            .execute(
                "
                INSERT INTO learning_runs(
                    processed_sources, changed_sources, opportunities_created, notes_json, completed_at
                )
                VALUES (?1, ?2, ?3, ?4, ?5)
                ",
                params![
                    run.processed_sources,
                    run.changed_sources,
                    run.opportunities_created,
                    notes_json,
                    completed_at,
                ],
            )
            .context("failed to record learning run")?;
        Ok(StoredLearningRun {
            id: connection.last_insert_rowid(),
            processed_sources: run.processed_sources,
            changed_sources: run.changed_sources,
            opportunities_created: run.opportunities_created,
            notes: run.notes,
            completed_at,
        })
    }

    pub fn latest_learning_run(&self) -> Result<Option<StoredLearningRun>> {
        let connection = self.connect()?;
        connection
            .query_row(
                "
                SELECT id, processed_sources, changed_sources, opportunities_created, notes_json, completed_at
                FROM learning_runs
                ORDER BY id DESC
                LIMIT 1
                ",
                [],
                |row| {
                    let notes =
                        serde_json::from_str::<Vec<String>>(&row.get::<_, String>(4)?).unwrap_or_default();
                    Ok(StoredLearningRun {
                        id: row.get(0)?,
                        processed_sources: row.get(1)?,
                        changed_sources: row.get(2)?,
                        opportunities_created: row.get(3)?,
                        notes,
                        completed_at: row.get(5)?,
                    })
                },
            )
            .optional()
            .context("failed to load latest learning run")
    }
}
