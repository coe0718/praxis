use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Utc};
use rusqlite::params;

use crate::memory::MemoryType;

use super::SqliteSessionStore;

const COLD_MEMORY_DECAY_MULTIPLIER: f32 = 0.97;
const COLD_MEMORY_FLOOR: f32 = 0.25;
const HOT_MEMORY_TTL_DAYS: i64 = 30;

pub(super) fn decay_cold_memories(store: &SqliteSessionStore, now: DateTime<Utc>) -> Result<usize> {
    let mut connection = store.get_connection()?;
    // Pull id, weight, and memory_type together so each type uses its own threshold.
    let candidates = {
        let mut statement = connection.prepare(
            "
            SELECT id, weight, COALESCE(memory_type, 'episodic')
            FROM cold_memories
            WHERE last_reinforced IS NOT NULL
            ",
        )?;

        statement
            .query_map([], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, f32>(1)?, row.get::<_, String>(2)?))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()
            .context("failed to collect cold memories for decay")?
    };

    let tx = connection.transaction().context("failed to begin memory decay transaction")?;
    let mut decayed = 0usize;
    for (id, weight, mt_str) in &candidates {
        let memory_type = MemoryType::parse(mt_str);
        let threshold = (now - Duration::days(memory_type.decay_days())).to_rfc3339();

        let is_stale: bool = tx
            .query_row(
                "SELECT 1 FROM cold_memories WHERE id = ?1 AND datetime(last_reinforced) <= datetime(?2)",
                params![id, threshold],
                |_| Ok(true),
            )
            .unwrap_or(false);

        if is_stale {
            let next_weight = (*weight * COLD_MEMORY_DECAY_MULTIPLIER).max(COLD_MEMORY_FLOOR);
            tx.execute(
                "UPDATE cold_memories SET weight = ?1 WHERE id = ?2",
                params![next_weight, id],
            )
            .with_context(|| format!("failed to decay cold memory {id}"))?;
            decayed += 1;
        }
    }
    tx.commit().context("failed to commit memory decay transaction")?;

    Ok(decayed)
}

/// Delete hot memories past their TTL (default 30 days without access).
pub(super) fn expire_hot_memories(store: &SqliteSessionStore, now: DateTime<Utc>) -> Result<usize> {
    let cutoff = (now - Duration::days(HOT_MEMORY_TTL_DAYS)).to_rfc3339();
    let connection = store.get_connection()?;

    let deleted = connection
        .execute(
            "DELETE FROM hot_memories
             WHERE datetime(last_accessed) <= datetime(?1)
             AND datetime(created_at) <= datetime(?1)",
            params![cutoff],
        )
        .context("failed to expire hot memories")?;

    if deleted > 0 {
        connection
            .execute("DELETE FROM hot_fts WHERE rowid NOT IN (SELECT id FROM hot_memories)", [])
            .context("failed to cleanup expired hot memory FTS entries")?;
    }

    if deleted > 0 {
        log::info!("memory decay: expired {deleted} hot memories (last accessed before {cutoff})");
    }

    Ok(deleted)
}

/// Demote cold memories below the floor weight back to hot.
pub(super) fn demote_cold_to_hot(store: &SqliteSessionStore, now: DateTime<Utc>) -> Result<usize> {
    let ninety_days_ago = (now - Duration::days(90)).to_rfc3339();
    let mut connection = store.get_connection()?;

    let candidates: Vec<(i64, String, String)> = {
        let mut stmt = connection
            .prepare(
                "SELECT id, content, tags
                 FROM cold_memories
                 WHERE weight <= 0.25
                 AND datetime(last_reinforced) <= datetime(?1)",
            )
            .context("failed to prepare cold memory demotion query")?;

        stmt.query_map(params![ninety_days_ago], |row| {
            Ok((row.get(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?))
        })?
        .collect::<std::result::Result<Vec<_>, _>>()
        .context("failed to collect demotion candidates")?
    };

    if candidates.is_empty() {
        return Ok(0);
    }

    let mut demoted = 0usize;
    let tx = connection.transaction().context("failed to begin demotion transaction")?;

    for (id, content, tags) in &candidates {
        let result = tx.execute(
            "INSERT INTO hot_memories(content, summary, importance, tags, last_accessed, access_count, expires_at, memory_type)
             VALUES (?1, 'Archived: demoted from cold storage', 0.3, ?2, datetime('now'), 0, NULL, 'episodic')",
            params![format!("[archived] {content}"), tags],
        );

        match result {
            Ok(_) => {
                if let Err(e) = tx.execute("DELETE FROM cold_memories WHERE id = ?1", params![id]) {
                    log::warn!("memory decay: failed to delete demoted cold memory {id}: {e}");
                    continue;
                }
                demoted += 1;
            }
            Err(e) => {
                log::warn!("memory decay: failed to insert demoted hot memory: {e}");
            }
        }
    }

    if demoted > 0 {
        let _ = tx
            .execute("DELETE FROM cold_fts WHERE rowid NOT IN (SELECT id FROM cold_memories)", []);
        let _ = tx.execute(
            "INSERT OR IGNORE INTO hot_fts(rowid, content, summary, tags)
             SELECT id, content, COALESCE(summary, ''), tags
             FROM hot_memories
             WHERE id NOT IN (SELECT rowid FROM hot_fts)",
            [],
        );
    }

    tx.commit().context("failed to commit demotion transaction")?;
    log::info!("memory decay: demoted {demoted} cold memories to hot (one last chance)");

    Ok(demoted)
}
