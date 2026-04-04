use std::{fs, path::Path};

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{Connection, OptionalExtension, params};

use crate::state::SessionState;

#[derive(Debug, Clone)]
pub struct SessionSnapshot {
    pub session_id: Option<i64>,
    pub started_at: String,
    pub phase: String,
    pub checkpoint: String,
    pub recorded_at: String,
    pub state: SessionState,
}

pub fn record_snapshot(db_path: &Path, state: &SessionState, checkpoint: &str) -> Result<()> {
    let connection = connect(db_path)?;
    connection
        .execute(
            "
            INSERT INTO session_snapshots(session_started_at, phase, checkpoint, state_json, recorded_at)
            VALUES (?1, ?2, ?3, ?4, ?5)
            ",
            params![
                state.started_at.to_rfc3339(),
                state.current_phase.to_string(),
                checkpoint,
                serde_json::to_string(state).context("failed to serialize session snapshot")?,
                Utc::now().to_rfc3339(),
            ],
        )
        .context("failed to insert session snapshot")?;
    Ok(())
}

pub fn attach_session_id(db_path: &Path, started_at: DateTime<Utc>, session_id: i64) -> Result<()> {
    let connection = connect(db_path)?;
    connection
        .execute(
            "
            UPDATE session_snapshots
            SET session_id = ?2
            WHERE session_started_at = ?1
            ",
            params![started_at.to_rfc3339(), session_id],
        )
        .context("failed to attach session id to snapshots")?;
    Ok(())
}

pub fn latest_started_at(db_path: &Path) -> Result<Option<String>> {
    let connection = connect(db_path)?;
    connection
        .query_row(
            "SELECT session_started_at FROM session_snapshots ORDER BY id DESC LIMIT 1",
            [],
            |row| row.get(0),
        )
        .optional()
        .context("failed to load latest snapshot session")
}

pub fn load_snapshots(db_path: &Path, started_at: &str) -> Result<Vec<SessionSnapshot>> {
    let connection = connect(db_path)?;
    let mut statement = connection
        .prepare(
            "
            SELECT session_id, session_started_at, phase, checkpoint, state_json, recorded_at
            FROM session_snapshots
            WHERE session_started_at = ?1
            ORDER BY id ASC
            ",
        )
        .context("failed to prepare snapshot query")?;
    let rows = statement
        .query_map(params![started_at], |row| {
            let state_json: String = row.get(4)?;
            let state = serde_json::from_str::<SessionState>(&state_json).map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(
                    4,
                    rusqlite::types::Type::Text,
                    Box::new(error),
                )
            })?;
            Ok(SessionSnapshot {
                session_id: row.get(0)?,
                started_at: row.get(1)?,
                phase: row.get(2)?,
                checkpoint: row.get(3)?,
                state,
                recorded_at: row.get(5)?,
            })
        })
        .context("failed to execute snapshot query")?;

    rows.collect::<std::result::Result<Vec<_>, _>>()
        .context("failed to collect session snapshots")
}

fn connect(path: &Path) -> Result<Connection> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    Connection::open(path).with_context(|| format!("failed to open {}", path.display()))
}
