use anyhow::{Context, Result};
use rusqlite::params;

use crate::{
    memory::{NewDoNotRepeat, NewKnownBug, StoredDoNotRepeat, StoredKnownBug},
    storage::{OperationalMemoryCounts, OperationalMemoryStore},
};

use super::SqliteSessionStore;

/// Escape SQL LIKE wildcards (`%` and `_`) in a user-supplied query string.
fn escape_like(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '%' => out.push_str("\\%"),
            '_' => out.push_str("\\_"),
            '\\' => out.push_str("\\\\"),
            c => out.push(c),
        }
    }
    out
}

impl OperationalMemoryStore for SqliteSessionStore {
    fn record_do_not_repeat(&self, entry: NewDoNotRepeat) -> Result<StoredDoNotRepeat> {
        let connection = self.connect()?;
        let tags = serde_json::to_string(&entry.tags).context("failed to serialize note tags")?;
        connection
            .execute(
                "
                INSERT INTO do_not_repeat(statement, tags, source_session_id, severity, expires_at)
                VALUES (?1, ?2, ?3, ?4, ?5)
                ",
                params![
                    entry.statement,
                    tags,
                    entry.source_session_id,
                    entry.severity,
                    entry.expires_at,
                ],
            )
            .context("failed to insert do-not-repeat entry")?;
        Ok(StoredDoNotRepeat {
            id: connection.last_insert_rowid(),
            statement: entry.statement,
            tags: entry.tags,
            severity: entry.severity,
        })
    }

    fn recent_do_not_repeat(&self, limit: usize) -> Result<Vec<StoredDoNotRepeat>> {
        let connection = self.connect()?;
        load_do_not_repeat(
            &connection,
            "
            SELECT id, statement, tags, severity
            FROM do_not_repeat
            ORDER BY id DESC
            LIMIT ?1
            ",
            params![limit as i64],
        )
    }

    fn search_do_not_repeat(&self, query: &str, limit: usize) -> Result<Vec<StoredDoNotRepeat>> {
        let connection = self.connect()?;
        let needle = format!("%{}%", escape_like(&query.to_lowercase()));
        load_do_not_repeat(
            &connection,
            "
            SELECT id, statement, tags, severity
            FROM do_not_repeat
            WHERE LOWER(statement) LIKE ?1 OR LOWER(tags) LIKE ?1
            ORDER BY id DESC
            LIMIT ?2
            ",
            params![needle, limit as i64],
        )
    }

    fn record_known_bug(&self, entry: NewKnownBug) -> Result<StoredKnownBug> {
        let connection = self.connect()?;
        let tags = serde_json::to_string(&entry.tags).context("failed to serialize bug tags")?;
        connection
            .execute(
                "
                INSERT INTO known_bugs(signature, symptoms, fix_summary, tags, source_session_id, resolved_at)
                VALUES (?1, ?2, ?3, ?4, ?5, CURRENT_TIMESTAMP)
                ",
                params![
                    entry.signature,
                    entry.symptoms,
                    entry.fix_summary,
                    tags,
                    entry.source_session_id,
                ],
            )
            .context("failed to insert known bug")?;
        Ok(StoredKnownBug {
            id: connection.last_insert_rowid(),
            signature: entry.signature,
            symptoms: entry.symptoms,
            fix_summary: entry.fix_summary,
            tags: entry.tags,
        })
    }

    fn recent_known_bugs(&self, limit: usize) -> Result<Vec<StoredKnownBug>> {
        let connection = self.connect()?;
        load_known_bugs(
            &connection,
            "
            SELECT id, signature, symptoms, fix_summary, tags
            FROM known_bugs
            ORDER BY id DESC
            LIMIT ?1
            ",
            params![limit as i64],
        )
    }

    fn search_known_bugs(&self, query: &str, limit: usize) -> Result<Vec<StoredKnownBug>> {
        let connection = self.connect()?;
        let needle = format!("%{}%", escape_like(&query.to_lowercase()));
        load_known_bugs(
            &connection,
            "
            SELECT id, signature, symptoms, fix_summary, tags
            FROM known_bugs
            WHERE LOWER(signature) LIKE ?1
               OR LOWER(symptoms) LIKE ?1
               OR LOWER(fix_summary) LIKE ?1
               OR LOWER(tags) LIKE ?1
            ORDER BY id DESC
            LIMIT ?2
            ",
            params![needle, limit as i64],
        )
    }

    fn operational_memory_counts(&self) -> Result<OperationalMemoryCounts> {
        let connection = self.connect()?;
        Ok(OperationalMemoryCounts {
            do_not_repeat: connection
                .query_row("SELECT COUNT(*) FROM do_not_repeat", [], |row| row.get(0))
                .context("failed to count do-not-repeat entries")?,
            known_bugs: connection
                .query_row("SELECT COUNT(*) FROM known_bugs", [], |row| row.get(0))
                .context("failed to count known bugs")?,
        })
    }
}

fn load_do_not_repeat<P>(
    connection: &rusqlite::Connection,
    sql: &str,
    params: P,
) -> Result<Vec<StoredDoNotRepeat>>
where
    P: rusqlite::Params,
{
    let mut statement = connection
        .prepare(sql)
        .context("failed to prepare note query")?;
    let rows = statement
        .query_map(params, |row| {
            let tags =
                serde_json::from_str::<Vec<String>>(&row.get::<_, String>(2)?).unwrap_or_default();
            Ok(StoredDoNotRepeat {
                id: row.get(0)?,
                statement: row.get(1)?,
                tags,
                severity: row.get(3)?,
            })
        })
        .context("failed to query do-not-repeat entries")?;
    rows.collect::<std::result::Result<Vec<_>, _>>()
        .context("failed to load do-not-repeat entries")
}

fn load_known_bugs<P>(
    connection: &rusqlite::Connection,
    sql: &str,
    params: P,
) -> Result<Vec<StoredKnownBug>>
where
    P: rusqlite::Params,
{
    let mut statement = connection
        .prepare(sql)
        .context("failed to prepare bug query")?;
    let rows = statement
        .query_map(params, |row| {
            let tags =
                serde_json::from_str::<Vec<String>>(&row.get::<_, String>(4)?).unwrap_or_default();
            Ok(StoredKnownBug {
                id: row.get(0)?,
                signature: row.get(1)?,
                symptoms: row.get(2)?,
                fix_summary: row.get(3)?,
                tags,
            })
        })
        .context("failed to query known bugs")?;
    rows.collect::<std::result::Result<Vec<_>, _>>()
        .context("failed to load known bugs")
}
