use anyhow::{Context, Result};
use rusqlite::{OptionalExtension, params};

use crate::storage::{SessionRecord, StoredSession};

use super::SqliteSessionStore;

pub(super) fn record_session(
    store: &SqliteSessionStore,
    record: &SessionRecord,
) -> Result<StoredSession> {
    let connection = store.connect()?;
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

pub(super) fn last_session(store: &SqliteSessionStore) -> Result<Option<StoredSession>> {
    if !store.path.exists() {
        return Ok(None);
    }

    let connection = store.connect()?;
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

fn next_session_number(connection: &rusqlite::Connection, day: i64) -> Result<i64> {
    connection
        .query_row(
            "SELECT COALESCE(MAX(session_num), 0) + 1 FROM sessions WHERE day = ?1",
            params![day],
            |row| row.get(0),
        )
        .context("failed to calculate next session number")
}
