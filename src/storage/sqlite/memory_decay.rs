use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Utc};
use rusqlite::params;

use crate::memory::MemoryType;

use super::SqliteSessionStore;

const COLD_MEMORY_DECAY_MULTIPLIER: f32 = 0.97;
const COLD_MEMORY_FLOOR: f32 = 0.25;

pub(super) fn decay_cold_memories(store: &SqliteSessionStore, now: DateTime<Utc>) -> Result<usize> {
    let connection = store.connect()?;

    // Pull id, weight, and memory_type together so each type uses its own threshold.
    let mut statement = connection.prepare(
        "
        SELECT id, weight, COALESCE(memory_type, 'episodic')
        FROM cold_memories
        WHERE last_reinforced IS NOT NULL
        ",
    )?;

    let candidates = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, f32>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()
        .context("failed to collect cold memories for decay")?;

    let mut decayed = 0usize;
    for (id, weight, mt_str) in &candidates {
        let memory_type = MemoryType::parse(mt_str);
        let threshold = (now - Duration::days(memory_type.decay_days())).to_rfc3339();

        let is_stale: bool = connection
            .query_row(
                "SELECT 1 FROM cold_memories WHERE id = ?1 AND datetime(last_reinforced) <= datetime(?2)",
                params![id, threshold],
                |_| Ok(true),
            )
            .unwrap_or(false);

        if is_stale {
            let next_weight = (*weight * COLD_MEMORY_DECAY_MULTIPLIER).max(COLD_MEMORY_FLOOR);
            connection
                .execute(
                    "UPDATE cold_memories SET weight = ?1 WHERE id = ?2",
                    params![next_weight, id],
                )
                .with_context(|| format!("failed to decay cold memory {id}"))?;
            decayed += 1;
        }
    }

    Ok(decayed)
}
