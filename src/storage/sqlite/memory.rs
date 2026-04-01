use anyhow::{Context, Result};
use chrono::Utc;
use rusqlite::params;

use crate::memory::{MemoryStore, MemoryTier, NewColdMemory, NewHotMemory, StoredMemory};

use super::SqliteSessionStore;

impl MemoryStore for SqliteSessionStore {
    fn insert_hot_memory(&self, memory: NewHotMemory) -> Result<StoredMemory> {
        let connection = self.connect()?;
        let tags = serde_json::to_string(&memory.tags).context("failed to serialize tags")?;

        connection
            .execute(
                "
                INSERT INTO hot_memories(content, summary, importance, tags, last_accessed, access_count, expires_at)
                VALUES (?1, ?2, ?3, ?4, ?5, 0, ?6)
                ",
                params![
                    memory.content,
                    memory.summary,
                    memory.importance,
                    tags,
                    Utc::now().to_rfc3339(),
                    memory.expires_at,
                ],
            )
            .context("failed to insert hot memory")?;

        let id = connection.last_insert_rowid();
        connection
            .execute(
                "INSERT INTO hot_fts(rowid, content, summary, tags) VALUES (?1, ?2, ?3, ?4)",
                params![id, memory.content, memory.summary, tags],
            )
            .context("failed to index hot memory")?;

        Ok(StoredMemory {
            id,
            tier: MemoryTier::Hot,
            content: memory.content,
            summary: memory.summary,
            tags: memory.tags,
            score: memory.importance,
        })
    }

    fn insert_cold_memory(&self, memory: NewColdMemory) -> Result<StoredMemory> {
        let connection = self.connect()?;
        let tags = serde_json::to_string(&memory.tags).context("failed to serialize tags")?;
        let source_ids =
            serde_json::to_string(&memory.source_ids).context("failed to serialize source ids")?;
        let contradicts = serde_json::to_string(&memory.contradicts)
            .context("failed to serialize contradiction ids")?;

        connection
            .execute(
                "
                INSERT INTO cold_memories(content, weight, tags, source_ids, contradicts, last_reinforced)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                ",
                params![
                    memory.content,
                    memory.weight,
                    tags,
                    source_ids,
                    contradicts,
                    Utc::now().to_rfc3339(),
                ],
            )
            .context("failed to insert cold memory")?;

        let id = connection.last_insert_rowid();
        connection
            .execute(
                "INSERT INTO cold_fts(rowid, content, tags) VALUES (?1, ?2, ?3)",
                params![id, memory.content, tags],
            )
            .context("failed to index cold memory")?;

        Ok(StoredMemory {
            id,
            tier: MemoryTier::Cold,
            content: memory.content,
            summary: None,
            tags: memory.tags,
            score: memory.weight,
        })
    }

    fn recent_hot_memories(&self, limit: usize) -> Result<Vec<StoredMemory>> {
        let connection = self.connect()?;
        let mut statement = connection.prepare(
            "
            SELECT id, content, summary, tags, importance
            FROM hot_memories
            ORDER BY datetime(created_at) DESC, id DESC
            LIMIT ?1
            ",
        )?;

        let rows = statement.query_map(params![limit as i64], |row| {
            Ok(StoredMemory {
                id: row.get(0)?,
                tier: MemoryTier::Hot,
                content: row.get(1)?,
                summary: row.get(2)?,
                tags: parse_tags(row.get::<_, String>(3)?)?,
                score: row.get(4)?,
            })
        })?;

        rows.collect::<rusqlite::Result<Vec<_>>>()
            .context("failed to load recent hot memories")
    }

    fn strongest_cold_memories(&self, limit: usize) -> Result<Vec<StoredMemory>> {
        let connection = self.connect()?;
        let mut statement = connection.prepare(
            "
            SELECT id, content, tags, weight
            FROM cold_memories
            ORDER BY weight DESC, id DESC
            LIMIT ?1
            ",
        )?;

        let rows = statement.query_map(params![limit as i64], |row| {
            Ok(StoredMemory {
                id: row.get(0)?,
                tier: MemoryTier::Cold,
                content: row.get(1)?,
                summary: None,
                tags: parse_tags(row.get::<_, String>(2)?)?,
                score: row.get(3)?,
            })
        })?;

        rows.collect::<rusqlite::Result<Vec<_>>>()
            .context("failed to load strongest cold memories")
    }

    fn search_memories(&self, query: &str, limit: usize) -> Result<Vec<StoredMemory>> {
        if query.trim().is_empty() {
            return Ok(Vec::new());
        }

        let connection = self.connect()?;
        let hot = search_hot_memories(&connection, query, limit)?;
        let cold = search_cold_memories(&connection, query, limit)?;
        let mut combined = hot.into_iter().chain(cold).collect::<Vec<_>>();
        combined.sort_by(|left, right| right.score.total_cmp(&left.score));
        combined.truncate(limit);
        Ok(combined)
    }
}

fn search_hot_memories(
    connection: &rusqlite::Connection,
    query: &str,
    limit: usize,
) -> Result<Vec<StoredMemory>> {
    let mut statement = connection.prepare(
        "
        SELECT hot_memories.id, hot_memories.content, hot_memories.summary, hot_memories.tags,
               hot_memories.importance, bm25(hot_fts) AS rank
        FROM hot_fts
        JOIN hot_memories ON hot_fts.rowid = hot_memories.id
        WHERE hot_fts MATCH ?1
        ORDER BY rank
        LIMIT ?2
        ",
    )?;

    let rows = statement.query_map(params![query, limit as i64], |row| {
        let rank: f64 = row.get(5)?;
        let importance: f32 = row.get(4)?;
        Ok(StoredMemory {
            id: row.get(0)?,
            tier: MemoryTier::Hot,
            content: row.get(1)?,
            summary: row.get(2)?,
            tags: parse_tags(row.get::<_, String>(3)?)?,
            score: importance + relevance_score(rank),
        })
    })?;

    rows.collect::<rusqlite::Result<Vec<_>>>()
        .context("failed to search hot memories")
}

fn search_cold_memories(
    connection: &rusqlite::Connection,
    query: &str,
    limit: usize,
) -> Result<Vec<StoredMemory>> {
    let mut statement = connection.prepare(
        "
        SELECT cold_memories.id, cold_memories.content, cold_memories.tags,
               cold_memories.weight, bm25(cold_fts) AS rank
        FROM cold_fts
        JOIN cold_memories ON cold_fts.rowid = cold_memories.id
        WHERE cold_fts MATCH ?1
        ORDER BY rank
        LIMIT ?2
        ",
    )?;

    let rows = statement.query_map(params![query, limit as i64], |row| {
        let rank: f64 = row.get(4)?;
        let weight: f32 = row.get(3)?;
        Ok(StoredMemory {
            id: row.get(0)?,
            tier: MemoryTier::Cold,
            content: row.get(1)?,
            summary: None,
            tags: parse_tags(row.get::<_, String>(2)?)?,
            score: weight + relevance_score(rank),
        })
    })?;

    rows.collect::<rusqlite::Result<Vec<_>>>()
        .context("failed to search cold memories")
}

fn parse_tags(raw: String) -> rusqlite::Result<Vec<String>> {
    serde_json::from_str(&raw).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(
            raw.len(),
            rusqlite::types::Type::Text,
            Box::new(error),
        )
    })
}

fn relevance_score(rank: f64) -> f32 {
    1.0 / (1.0 + rank.abs() as f32)
}
