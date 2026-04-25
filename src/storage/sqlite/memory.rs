use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{OptionalExtension, params};

use crate::memory::{
    ConsolidationSummary, MemoryStore, MemoryTier, MemoryType, NewColdMemory, NewHotMemory,
    StoredMemory, to_fts_query, vector,
};

use super::SqliteSessionStore;

impl MemoryStore for SqliteSessionStore {
    fn insert_hot_memory(&self, memory: NewHotMemory) -> Result<StoredMemory> {
        let mut connection = self.connect()?;
        let tags = serde_json::to_string(&memory.tags).context("failed to serialize tags")?;

        let tx = connection.transaction().context("failed to begin hot memory transaction")?;
        tx.execute(
            "
                INSERT INTO hot_memories(content, summary, importance, tags, last_accessed, access_count, expires_at, memory_type)
                VALUES (?1, ?2, ?3, ?4, ?5, 0, ?6, ?7)
                ",
            params![
                memory.content,
                memory.summary,
                memory.importance,
                tags,
                Utc::now().to_rfc3339(),
                memory.expires_at,
                memory.memory_type.as_str(),
            ],
        )
        .context("failed to insert hot memory")?;

        let id = tx.last_insert_rowid();
        tx.execute(
            "INSERT INTO hot_fts(rowid, content, summary, tags) VALUES (?1, ?2, ?3, ?4)",
            params![id, memory.content, memory.summary, tags],
        )
        .context("failed to index hot memory")?;
        tx.commit().context("failed to commit hot memory transaction")?;

        Ok(StoredMemory {
            id,
            tier: MemoryTier::Hot,
            content: memory.content,
            summary: memory.summary,
            tags: memory.tags,
            score: memory.importance,
            memory_type: memory.memory_type,
        })
    }

    fn insert_cold_memory(&self, memory: NewColdMemory) -> Result<StoredMemory> {
        let mut connection = self.connect()?;
        let tags = serde_json::to_string(&memory.tags).context("failed to serialize tags")?;
        let source_ids =
            serde_json::to_string(&memory.source_ids).context("failed to serialize source ids")?;
        let contradicts = serde_json::to_string(&memory.contradicts)
            .context("failed to serialize contradiction ids")?;

        let tx = connection.transaction().context("failed to begin cold memory transaction")?;
        tx.execute(
            "
                INSERT INTO cold_memories(content, weight, tags, source_ids, contradicts, last_reinforced, memory_type)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                ",
            params![
                memory.content,
                memory.weight,
                tags,
                source_ids,
                contradicts,
                Utc::now().to_rfc3339(),
                memory.memory_type.as_str(),
            ],
        )
        .context("failed to insert cold memory")?;

        let id = tx.last_insert_rowid();
        tx.execute(
            "INSERT INTO cold_fts(rowid, content, tags) VALUES (?1, ?2, ?3)",
            params![id, memory.content, tags],
        )
        .context("failed to index cold memory")?;
        tx.commit().context("failed to commit cold memory transaction")?;

        Ok(StoredMemory {
            id,
            tier: MemoryTier::Cold,
            content: memory.content,
            summary: None,
            tags: memory.tags,
            score: memory.weight,
            memory_type: memory.memory_type,
        })
    }

    fn recent_hot_memories(&self, limit: usize) -> Result<Vec<StoredMemory>> {
        let connection = self.connect()?;
        let mut statement = connection.prepare(
            "
            SELECT id, content, summary, tags, importance, COALESCE(memory_type, 'episodic')
            FROM hot_memories
            ORDER BY datetime(created_at) DESC, id DESC
            LIMIT ?1
            ",
        )?;

        let rows = statement.query_map(params![limit as i64], |row| {
            let mt: String = row.get(5)?;
            Ok(StoredMemory {
                id: row.get(0)?,
                tier: MemoryTier::Hot,
                content: row.get(1)?,
                summary: row.get(2)?,
                tags: parse_tags(row.get::<_, String>(3)?)?,
                score: row.get(4)?,
                memory_type: MemoryType::parse(&mt),
            })
        })?;

        rows.collect::<rusqlite::Result<Vec<_>>>()
            .context("failed to load recent hot memories")
    }

    fn strongest_cold_memories(&self, limit: usize) -> Result<Vec<StoredMemory>> {
        let connection = self.connect()?;
        let mut statement = connection.prepare(
            "
            SELECT id, content, tags, weight, COALESCE(memory_type, 'episodic')
            FROM cold_memories
            ORDER BY weight DESC, id DESC
            LIMIT ?1
            ",
        )?;

        let rows = statement.query_map(params![limit as i64], |row| {
            let mt: String = row.get(4)?;
            Ok(StoredMemory {
                id: row.get(0)?,
                tier: MemoryTier::Cold,
                content: row.get(1)?,
                summary: None,
                tags: parse_tags(row.get::<_, String>(2)?)?,
                score: row.get(3)?,
                memory_type: MemoryType::parse(&mt),
            })
        })?;

        rows.collect::<rusqlite::Result<Vec<_>>>()
            .context("failed to load strongest cold memories")
    }

    fn search_memories(&self, query: &str, limit: usize) -> Result<Vec<StoredMemory>> {
        let Some(query) = to_fts_query(query) else {
            return Ok(Vec::new());
        };

        let connection = self.connect()?;
        let half = limit.div_ceil(2);
        let hot = search_hot_memories(&connection, &query, half)?;
        let cold = search_cold_memories(&connection, &query, half)?;
        let mut combined = hot.into_iter().chain(cold).collect::<Vec<_>>();
        combined.sort_by(|left, right| right.score.total_cmp(&left.score));
        combined.truncate(limit);
        Ok(combined)
    }

    fn decay_cold_memories(&self, now: DateTime<Utc>) -> Result<usize> {
        super::memory_decay::decay_cold_memories(self, now)
    }

    fn consolidate_memories(&self, now: DateTime<Utc>) -> Result<ConsolidationSummary> {
        super::memory_consolidation::consolidate_memories(self, now)
    }

    fn get_memory(&self, id: i64) -> Result<Option<StoredMemory>> {
        let connection = self.connect()?;

        // Try hot first.
        let hot: Option<StoredMemory> = connection
            .query_row(
                "SELECT id, content, summary, tags, importance, COALESCE(memory_type, 'episodic') FROM hot_memories WHERE id = ?1",
                params![id],
                |row| {
                    let mt: String = row.get(5)?;
                    Ok(StoredMemory {
                        id: row.get(0)?,
                        tier: MemoryTier::Hot,
                        content: row.get(1)?,
                        summary: row.get(2)?,
                        tags: parse_tags(row.get::<_, String>(3)?)?,
                        score: row.get(4)?,
                        memory_type: MemoryType::parse(&mt),
                    })
                },
            )
            .optional()
            .context("failed to query hot memory by id")?;

        if hot.is_some() {
            return Ok(hot);
        }

        // Try cold.
        let cold: Option<StoredMemory> = connection
            .query_row(
                "SELECT id, content, tags, weight, COALESCE(memory_type, 'episodic') FROM cold_memories WHERE id = ?1",
                params![id],
                |row| {
                    let mt: String = row.get(4)?;
                    Ok(StoredMemory {
                        id: row.get(0)?,
                        tier: MemoryTier::Cold,
                        content: row.get(1)?,
                        summary: None,
                        tags: parse_tags(row.get::<_, String>(2)?)?,
                        score: row.get(3)?,
                        memory_type: MemoryType::parse(&mt),
                    })
                },
            )
            .optional()
            .context("failed to query cold memory by id")?;

        Ok(cold)
    }

    fn boost_memory(&self, id: i64) -> Result<bool> {
        let connection = self.connect()?;

        // Try hot first.
        let hot_rows = connection
            .execute(
                "UPDATE hot_memories SET importance = MIN(importance + 0.2, 1.0),
                         last_accessed = datetime('now'),
                         access_count = access_count + 1
                  WHERE id = ?1",
                params![id],
            )
            .context("failed to boost hot memory")?;

        if hot_rows > 0 {
            return Ok(true);
        }

        // Try cold.
        let cold_rows = connection
            .execute(
                "UPDATE cold_memories SET weight = MIN(weight + 0.2, 2.0),
                         last_reinforced = datetime('now')
                  WHERE id = ?1",
                params![id],
            )
            .context("failed to boost cold memory")?;

        Ok(cold_rows > 0)
    }

    fn forget_memory(&self, id: i64) -> Result<bool> {
        let connection = self.connect()?;

        let hot_rows = connection
            .execute("DELETE FROM hot_memories WHERE id = ?1", params![id])
            .context("failed to delete hot memory")?;
        if hot_rows > 0 {
            // FTS shadow table uses the same rowid.
            if let Err(e) = connection.execute("DELETE FROM hot_fts WHERE rowid = ?1", params![id])
            {
                log::warn!("failed to remove memory {id} from hot_fts index: {e}");
            }
            return Ok(true);
        }

        let cold_rows = connection
            .execute("DELETE FROM cold_memories WHERE id = ?1", params![id])
            .context("failed to delete cold memory")?;
        if cold_rows > 0 {
            if let Err(e) = connection.execute("DELETE FROM cold_fts WHERE rowid = ?1", params![id])
            {
                log::warn!("failed to remove memory {id} from cold_fts index: {e}");
            }
            return Ok(true);
        }

        Ok(false)
    }

    fn search_memories_semantic(&self, query: &str, limit: usize) -> Result<Vec<StoredMemory>> {
        let query_embedding = vector::generate_embedding(query);

        let connection = self.connect()?;

        // Load all memories with embeddings from both tiers.
        let mut candidates: Vec<(StoredMemory, f32)> = Vec::new();

        // Hot memories with embeddings.
        let mut hot_stmt = connection.prepare(
            "SELECT id, content, summary, tags, importance, embedding, COALESCE(memory_type, 'episodic')
             FROM hot_memories WHERE embedding IS NOT NULL",
        )?;
        let hot_rows = hot_stmt.query_map([], |row| {
            let blob: Vec<u8> = row.get(5)?;
            let emb = vector::blob_to_embedding(&blob).unwrap_or_default();
            let sim = vector::cosine_similarity(&query_embedding, &emb);
            let mt: String = row.get(6)?;
            let m = StoredMemory {
                id: row.get(0)?,
                tier: MemoryTier::Hot,
                content: row.get(1)?,
                summary: row.get(2)?,
                tags: parse_tags(row.get::<_, String>(3)?).unwrap_or_default(),
                score: row.get(4)?,
                memory_type: MemoryType::parse(&mt),
            };
            Ok((m, sim))
        })?;
        for item in hot_rows.flatten() {
            candidates.push(item);
        }

        // Cold memories with embeddings.
        let mut cold_stmt = connection.prepare(
            "SELECT id, content, tags, weight, embedding, COALESCE(memory_type, 'episodic')
             FROM cold_memories WHERE embedding IS NOT NULL",
        )?;
        let cold_rows = cold_stmt.query_map([], |row| {
            let blob: Vec<u8> = row.get(4)?;
            let emb = vector::blob_to_embedding(&blob).unwrap_or_default();
            let sim = vector::cosine_similarity(&query_embedding, &emb);
            let mt: String = row.get(5)?;
            let m = StoredMemory {
                id: row.get(0)?,
                tier: MemoryTier::Cold,
                content: row.get(1)?,
                summary: None,
                tags: parse_tags(row.get::<_, String>(2)?).unwrap_or_default(),
                score: row.get(3)?,
                memory_type: MemoryType::parse(&mt),
            };
            Ok((m, sim))
        })?;
        for item in cold_rows.flatten() {
            candidates.push(item);
        }

        // If no embeddings exist, fall back to FTS5 keyword search.
        if candidates.is_empty() {
            return self.search_memories(query, limit);
        }

        // Hybrid score: 60% vector similarity + 40% base score (importance/weight).
        candidates.sort_by(|a, b| {
            let score_a = 0.6 * a.1 + 0.4 * a.0.score;
            let score_b = 0.6 * b.1 + 0.4 * b.0.score;
            score_b.total_cmp(&score_a)
        });

        let mut results: Vec<StoredMemory> = candidates
            .into_iter()
            .take(limit)
            .map(|(mut m, sim)| {
                m.score = 0.6 * sim + 0.4 * m.score;
                m
            })
            .collect();

        // Also include any FTS5 keyword matches that didn't have embeddings.
        let fts_results = self.search_memories(query, limit)?;
        let existing_ids: std::collections::HashSet<i64> = results.iter().map(|m| m.id).collect();
        for m in fts_results {
            if !existing_ids.contains(&m.id) && results.len() < limit {
                results.push(m);
            }
        }

        Ok(results)
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
               hot_memories.importance, bm25(hot_fts) AS rank,
               COALESCE(hot_memories.memory_type, 'episodic')
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
        let mt: String = row.get(6)?;
        Ok(StoredMemory {
            id: row.get(0)?,
            tier: MemoryTier::Hot,
            content: row.get(1)?,
            summary: row.get(2)?,
            tags: parse_tags(row.get::<_, String>(3)?)?,
            score: importance + relevance_score(rank),
            memory_type: MemoryType::parse(&mt),
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
               cold_memories.weight, bm25(cold_fts) AS rank,
               COALESCE(cold_memories.memory_type, 'episodic')
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
        let mt: String = row.get(5)?;
        Ok(StoredMemory {
            id: row.get(0)?,
            tier: MemoryTier::Cold,
            content: row.get(1)?,
            summary: None,
            tags: parse_tags(row.get::<_, String>(2)?)?,
            score: weight + relevance_score(rank),
            memory_type: MemoryType::parse(&mt),
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
