use anyhow::{Context, Result, bail};
use chrono::Utc;
use rusqlite::{Connection, OptionalExtension, params};

use super::{SqliteSessionStore, schema_data};

pub(super) fn initialize(store: &SqliteSessionStore) -> Result<()> {
    let connection = store.connect()?;
    connection
        .execute_batch(schema_data::BASE_SCHEMA)
        .context("failed to initialize SQLite schema")?;
    ensure_session_column(&connection, "reviewer_passes", "INTEGER NOT NULL DEFAULT 0")?;
    ensure_session_column(&connection, "reviewer_failures", "INTEGER NOT NULL DEFAULT 0")?;
    ensure_session_column(&connection, "eval_passes", "INTEGER NOT NULL DEFAULT 0")?;
    ensure_session_column(&connection, "eval_failures", "INTEGER NOT NULL DEFAULT 0")?;
    ensure_session_column(&connection, "repeated_reads_avoided", "INTEGER NOT NULL DEFAULT 0")?;
    ensure_table_column(&connection, "opportunities", "goal_id", "TEXT")?;
    ensure_table_column(&connection, "approval_requests", "payload_json", "TEXT")?;
    ensure_table_column(
        &connection,
        "hot_memories",
        "memory_type",
        "TEXT NOT NULL DEFAULT 'episodic'",
    )?;
    ensure_table_column(
        &connection,
        "cold_memories",
        "memory_type",
        "TEXT NOT NULL DEFAULT 'episodic'",
    )?;
    ensure_table_column(&connection, "hot_memories", "embedding", "BLOB")?;
    ensure_table_column(&connection, "cold_memories", "embedding", "BLOB")?;

    connection
        .execute(
            "INSERT OR IGNORE INTO schema_migrations(version, applied_at) VALUES (?1, ?2)",
            params![schema_data::SCHEMA_VERSION, Utc::now().to_rfc3339()],
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
        .query_row("SELECT MAX(version) FROM schema_migrations", [], |row| row.get(0))
        .optional()
        .context("failed to query schema migrations")?;
    if version.unwrap_or_default() < schema_data::SCHEMA_VERSION {
        bail!(
            "expected schema migration version {} in {}",
            schema_data::SCHEMA_VERSION,
            store.path.display()
        );
    }

    for table_name in schema_data::EXPECTED_TABLES {
        let table: Option<String> = connection
            .query_row(
                "SELECT name FROM sqlite_master WHERE type = 'table' AND name = ?1",
                params![table_name],
                |row| row.get(0),
            )
            .optional()
            .with_context(|| format!("failed to validate {table_name} table"))?;

        if table.as_deref() != Some(*table_name) {
            bail!("{table_name} table is missing from {}", store.path.display());
        }
    }

    Ok(())
}

fn ensure_session_column(connection: &Connection, name: &str, definition: &str) -> Result<()> {
    validate_identifier(name)?;
    ensure_table_column(connection, "sessions", name, definition)
}

/// Validate that a SQL identifier (table/column name) contains only safe characters.
/// Prevents injection via format!() in DDL statements.
fn validate_identifier(name: &str) -> Result<()> {
    if name.is_empty() {
        bail!("SQL identifier must not be empty");
    }
    let mut chars = name.chars();
    let first = chars.next().unwrap();
    if !first.is_ascii_lowercase() && first != '_' {
        bail!("invalid SQL identifier: '{name}'");
    }
    for ch in chars {
        if !ch.is_ascii_lowercase() && !ch.is_ascii_digit() && ch != '_' {
            bail!("invalid SQL identifier: '{name}'");
        }
    }
    Ok(())
}

fn ensure_table_column(
    connection: &Connection,
    table: &str,
    name: &str,
    definition: &str,
) -> Result<()> {
    validate_identifier(table)?;
    validate_identifier(name)?;
    let mut statement = connection
        .prepare(&format!("PRAGMA table_info({table})"))
        .with_context(|| format!("failed to inspect {table} table"))?;
    let mut rows = statement.query([]).with_context(|| format!("failed to query {table} table"))?;

    while let Some(row) = rows.next().with_context(|| format!("failed to read {table} columns"))? {
        let column_name: String = row.get(1).context("failed to read column name")?;
        if column_name == name {
            return Ok(());
        }
    }

    connection
        .execute(&format!("ALTER TABLE {table} ADD COLUMN {name} {definition}"), [])
        .with_context(|| format!("failed to add {table}.{name}"))?;
    Ok(())
}
