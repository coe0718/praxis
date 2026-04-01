use std::{fs, path::PathBuf};

use anyhow::{Context, Result, bail};
use chrono::Utc;
use rusqlite::{Connection, OptionalExtension, params};

use super::{SessionRecord, SessionStore, StoredSession};

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
        let connection = self.connect()?;
        connection
            .execute_batch(
                "
                CREATE TABLE IF NOT EXISTS schema_migrations (
                    version INTEGER PRIMARY KEY,
                    applied_at TEXT NOT NULL
                );
                CREATE TABLE IF NOT EXISTS sessions (
                    id INTEGER PRIMARY KEY,
                    day INTEGER NOT NULL,
                    session_num INTEGER NOT NULL,
                    started_at TEXT NOT NULL,
                    ended_at TEXT NOT NULL,
                    tokens_used INTEGER NOT NULL DEFAULT 0,
                    goals_completed INTEGER NOT NULL DEFAULT 0,
                    goals_attempted INTEGER NOT NULL DEFAULT 0,
                    lines_written INTEGER NOT NULL DEFAULT 0,
                    memory_captures INTEGER NOT NULL DEFAULT 0,
                    loop_guard_triggers INTEGER NOT NULL DEFAULT 0,
                    reviewer_passes INTEGER NOT NULL DEFAULT 0,
                    reviewer_failures INTEGER NOT NULL DEFAULT 0,
                    phase_durations TEXT NOT NULL,
                    outcome TEXT NOT NULL,
                    selected_goal_id TEXT,
                    selected_goal_title TEXT,
                    selected_task TEXT,
                    action_summary TEXT NOT NULL
                );
                ",
            )
            .context("failed to initialize SQLite schema")?;

        connection
            .execute(
                "INSERT OR IGNORE INTO schema_migrations(version, applied_at) VALUES (?1, ?2)",
                params![1_i64, Utc::now().to_rfc3339()],
            )
            .context("failed to register schema migration")?;

        Ok(())
    }

    fn validate_schema(&self) -> Result<()> {
        if !self.path.exists() {
            bail!("database file {} does not exist", self.path.display());
        }

        let connection = self.connect()?;
        let version: Option<i64> = connection
            .query_row(
                "SELECT version FROM schema_migrations WHERE version = 1",
                [],
                |row| row.get(0),
            )
            .optional()
            .context("failed to query schema migrations")?;

        if version != Some(1) {
            bail!(
                "expected schema migration version 1 in {}",
                self.path.display()
            );
        }

        let sessions_table: Option<String> = connection
            .query_row(
                "SELECT name FROM sqlite_master WHERE type = 'table' AND name = 'sessions'",
                [],
                |row| row.get(0),
            )
            .optional()
            .context("failed to validate sessions table")?;

        if sessions_table.as_deref() != Some("sessions") {
            bail!("sessions table is missing from {}", self.path.display());
        }

        Ok(())
    }

    fn record_session(&self, record: &SessionRecord) -> Result<StoredSession> {
        let connection = self.connect()?;
        let session_num = next_session_number(&connection, record.day)?;
        let started_at = record.started_at.to_rfc3339();
        let ended_at = record.ended_at.to_rfc3339();

        connection
            .execute(
                "
                INSERT INTO sessions (
                    day, session_num, started_at, ended_at, phase_durations, outcome,
                    selected_goal_id, selected_goal_title, selected_task, action_summary
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
                ",
                params![
                    record.day,
                    session_num,
                    started_at,
                    ended_at,
                    record.phase_durations_json,
                    record.outcome,
                    record.selected_goal_id,
                    record.selected_goal_title,
                    record.selected_task,
                    record.action_summary,
                ],
            )
            .context("failed to insert session row")?;

        Ok(StoredSession {
            id: connection.last_insert_rowid(),
            day: record.day,
            session_num,
            started_at,
            ended_at,
            outcome: record.outcome.clone(),
            selected_goal_id: record.selected_goal_id.clone(),
            selected_goal_title: record.selected_goal_title.clone(),
            selected_task: record.selected_task.clone(),
            action_summary: record.action_summary.clone(),
        })
    }

    fn last_session(&self) -> Result<Option<StoredSession>> {
        if !self.path.exists() {
            return Ok(None);
        }

        let connection = self.connect()?;
        connection
            .query_row(
                "
                SELECT id, day, session_num, started_at, ended_at, outcome,
                       selected_goal_id, selected_goal_title, selected_task, action_summary
                FROM sessions
                ORDER BY id DESC
                LIMIT 1
                ",
                [],
                |row| {
                    Ok(StoredSession {
                        id: row.get(0)?,
                        day: row.get(1)?,
                        session_num: row.get(2)?,
                        started_at: row.get(3)?,
                        ended_at: row.get(4)?,
                        outcome: row.get(5)?,
                        selected_goal_id: row.get(6)?,
                        selected_goal_title: row.get(7)?,
                        selected_task: row.get(8)?,
                        action_summary: row.get(9)?,
                    })
                },
            )
            .optional()
            .context("failed to load the most recent session")
    }
}

fn next_session_number(connection: &Connection, day: i64) -> Result<i64> {
    connection
        .query_row(
            "SELECT COALESCE(MAX(session_num), 0) + 1 FROM sessions WHERE day = ?1",
            params![day],
            |row| row.get(0),
        )
        .context("failed to calculate next session number")
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;
    use tempfile::tempdir;

    use super::SqliteSessionStore;
    use crate::storage::{SessionRecord, SessionStore};

    #[test]
    fn initializes_and_records_sessions() {
        let temp = tempdir().unwrap();
        let store = SqliteSessionStore::new(temp.path().join("praxis.db"));
        store.initialize().unwrap();
        store.validate_schema().unwrap();

        let record = SessionRecord {
            day: 0,
            started_at: chrono::Utc.with_ymd_and_hms(2026, 3, 31, 12, 0, 0).unwrap(),
            ended_at: chrono::Utc.with_ymd_and_hms(2026, 3, 31, 12, 5, 0).unwrap(),
            outcome: "goal_selected".to_string(),
            selected_goal_id: Some("G-001".to_string()),
            selected_goal_title: Some("Ship foundation".to_string()),
            selected_task: None,
            action_summary: "Stub backend prepared the goal.".to_string(),
            phase_durations_json: "{\"orient\":0}".to_string(),
        };

        let stored = store.record_session(&record).unwrap();
        assert_eq!(stored.session_num, 1);
        assert_eq!(
            store.last_session().unwrap().unwrap().outcome,
            "goal_selected"
        );
    }
}
