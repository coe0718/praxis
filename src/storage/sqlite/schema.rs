use anyhow::{Context, Result, bail};
use chrono::Utc;
use rusqlite::{Connection, OptionalExtension, params};

use super::SqliteSessionStore;

pub(super) fn initialize(store: &SqliteSessionStore) -> Result<()> {
    let connection = store.connect()?;
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
            CREATE TABLE IF NOT EXISTS hot_memories (
                id INTEGER PRIMARY KEY,
                content TEXT NOT NULL,
                summary TEXT,
                importance REAL NOT NULL DEFAULT 0.5,
                tags TEXT NOT NULL DEFAULT '[]',
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                last_accessed TEXT,
                access_count INTEGER NOT NULL DEFAULT 0,
                expires_at TEXT
            );
            CREATE VIRTUAL TABLE IF NOT EXISTS hot_fts USING fts5(content, summary, tags);
            CREATE TABLE IF NOT EXISTS cold_memories (
                id INTEGER PRIMARY KEY,
                content TEXT NOT NULL,
                weight REAL NOT NULL DEFAULT 1.0,
                tags TEXT NOT NULL DEFAULT '[]',
                source_ids TEXT NOT NULL DEFAULT '[]',
                contradicts TEXT NOT NULL DEFAULT '[]',
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                last_reinforced TEXT
            );
            CREATE VIRTUAL TABLE IF NOT EXISTS cold_fts USING fts5(content, tags);
            CREATE TABLE IF NOT EXISTS approval_requests (
                id INTEGER PRIMARY KEY,
                tool_name TEXT NOT NULL,
                summary TEXT NOT NULL,
                requested_by TEXT NOT NULL,
                write_paths TEXT NOT NULL DEFAULT '[]',
                status TEXT NOT NULL,
                status_note TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS review_runs (
                id INTEGER PRIMARY KEY,
                session_id INTEGER NOT NULL,
                goal_id TEXT,
                status TEXT NOT NULL,
                summary TEXT NOT NULL,
                findings_json TEXT NOT NULL DEFAULT '[]',
                reviewed_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS eval_runs (
                id INTEGER PRIMARY KEY,
                session_id INTEGER NOT NULL,
                eval_id TEXT NOT NULL,
                eval_name TEXT NOT NULL,
                status TEXT NOT NULL,
                severity TEXT NOT NULL,
                summary TEXT NOT NULL,
                evaluated_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS session_snapshots (
                id INTEGER PRIMARY KEY,
                session_id INTEGER,
                session_started_at TEXT NOT NULL,
                phase TEXT NOT NULL,
                checkpoint TEXT NOT NULL,
                state_json TEXT NOT NULL,
                recorded_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS provider_attempts (
                id INTEGER PRIMARY KEY,
                session_id INTEGER NOT NULL,
                phase TEXT NOT NULL,
                provider TEXT NOT NULL,
                model TEXT NOT NULL,
                success INTEGER NOT NULL,
                input_tokens INTEGER NOT NULL DEFAULT 0,
                output_tokens INTEGER NOT NULL DEFAULT 0,
                estimated_cost_micros INTEGER NOT NULL DEFAULT 0,
                error TEXT
            );
            ",
        )
        .context("failed to initialize SQLite schema")?;
    ensure_session_column(&connection, "reviewer_passes", "INTEGER NOT NULL DEFAULT 0")?;
    ensure_session_column(
        &connection,
        "reviewer_failures",
        "INTEGER NOT NULL DEFAULT 0",
    )?;
    ensure_session_column(&connection, "eval_passes", "INTEGER NOT NULL DEFAULT 0")?;
    ensure_session_column(&connection, "eval_failures", "INTEGER NOT NULL DEFAULT 0")?;

    connection
        .execute(
            "INSERT OR IGNORE INTO schema_migrations(version, applied_at) VALUES (?1, ?2)",
            params![6_i64, Utc::now().to_rfc3339()],
        )
        .context("failed to register schema migration")?;
    Ok(())
}

pub(super) fn validate(store: &SqliteSessionStore) -> Result<()> {
    if !store.path.exists() {
        bail!("database file {} does not exist", store.path.display());
    }

    let connection = store.connect()?;
    let version: Option<i64> = connection
        .query_row("SELECT MAX(version) FROM schema_migrations", [], |row| {
            row.get(0)
        })
        .optional()
        .context("failed to query schema migrations")?;
    if version.unwrap_or_default() < 6 {
        bail!(
            "expected schema migration version 6 in {}",
            store.path.display()
        );
    }

    for table_name in [
        "sessions",
        "hot_memories",
        "hot_fts",
        "cold_memories",
        "cold_fts",
        "approval_requests",
        "review_runs",
        "eval_runs",
        "session_snapshots",
        "provider_attempts",
    ] {
        let table: Option<String> = connection
            .query_row(
                "SELECT name FROM sqlite_master WHERE type = 'table' AND name = ?1",
                params![table_name],
                |row| row.get(0),
            )
            .optional()
            .with_context(|| format!("failed to validate {table_name} table"))?;

        if table.as_deref() != Some(table_name) {
            bail!(
                "{table_name} table is missing from {}",
                store.path.display()
            );
        }
    }

    Ok(())
}

fn ensure_session_column(connection: &Connection, name: &str, definition: &str) -> Result<()> {
    let mut statement = connection
        .prepare("PRAGMA table_info(sessions)")
        .context("failed to inspect sessions table")?;
    let mut rows = statement
        .query([])
        .context("failed to query sessions table")?;

    while let Some(row) = rows.next().context("failed to read sessions columns")? {
        let column_name: String = row.get(1).context("failed to read column name")?;
        if column_name == name {
            return Ok(());
        }
    }

    connection
        .execute(
            &format!("ALTER TABLE sessions ADD COLUMN {name} {definition}"),
            [],
        )
        .with_context(|| format!("failed to add sessions.{name}"))?;
    Ok(())
}
