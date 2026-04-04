use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Utc};
use rusqlite::params;

use super::SqliteSessionStore;

const COLD_MEMORY_DECAY_DAYS: i64 = 90;
const COLD_MEMORY_DECAY_MULTIPLIER: f32 = 0.97;
const COLD_MEMORY_FLOOR: f32 = 0.25;

pub(super) fn decay_cold_memories(store: &SqliteSessionStore, now: DateTime<Utc>) -> Result<usize> {
    let connection = store.connect()?;
    let threshold = (now - Duration::days(COLD_MEMORY_DECAY_DAYS)).to_rfc3339();
    let mut statement = connection.prepare(
        "
        SELECT id, weight
        FROM cold_memories
        WHERE last_reinforced IS NOT NULL
          AND datetime(last_reinforced) <= datetime(?1)
        ",
    )?;
    let stale = statement
        .query_map(params![threshold], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, f32>(1)?))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()
        .context("failed to collect stale cold memories")?;

    for (id, weight) in &stale {
        let next_weight = (*weight * COLD_MEMORY_DECAY_MULTIPLIER).max(COLD_MEMORY_FLOOR);
        connection
            .execute(
                "UPDATE cold_memories SET weight = ?1 WHERE id = ?2",
                params![next_weight, id],
            )
            .with_context(|| format!("failed to decay cold memory {id}"))?;
    }

    Ok(stale.len())
}
