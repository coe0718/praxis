use anyhow::{Context, Result, bail};
use chrono::Utc;
use rusqlite::{OptionalExtension, params};

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
            ",
        )
        .context("failed to initialize SQLite schema")?;

    connection
        .execute(
            "INSERT OR IGNORE INTO schema_migrations(version, applied_at) VALUES (?1, ?2)",
            params![2_i64, Utc::now().to_rfc3339()],
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
    if version.unwrap_or_default() < 2 {
        bail!(
            "expected schema migration version 2 in {}",
            store.path.display()
        );
    }

    for table_name in [
        "sessions",
        "hot_memories",
        "hot_fts",
        "cold_memories",
        "cold_fts",
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
