use std::collections::{HashMap, HashSet};

use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Utc};
use rusqlite::params;

use crate::memory::{ConsolidationSummary, MemoryStore, MemoryType, NewColdMemory};

use super::SqliteSessionStore;

/// Minimum cluster size before hot memories are promoted to cold.
const MIN_CLUSTER_SIZE: usize = 3;

/// Hot memories must be at least this old before they are eligible for consolidation.
const MIN_AGE_DAYS: i64 = 7;

/// Cold memories at the weight floor beyond this multiple of their decay threshold are pruned.
const PRUNE_MULTIPLIER: i64 = 2;

/// Weight floor — must match memory_decay.rs.
const COLD_MEMORY_FLOOR: f32 = 0.25;

pub(super) fn consolidate_memories(
    store: &SqliteSessionStore,
    now: DateTime<Utc>,
) -> Result<ConsolidationSummary> {
    let consolidated = consolidate_hot_memories(store, now)?;
    let pruned = prune_cold_memories(store, now)?;
    Ok(ConsolidationSummary {
        consolidated,
        pruned,
    })
}

struct HotCandidate {
    id: i64,
    content: String,
    summary: Option<String>,
    tags: Vec<String>,
    importance: f32,
    memory_type: MemoryType,
}

/// Cluster eligible hot memories by tag and promote clusters of MIN_CLUSTER_SIZE+ to cold.
fn consolidate_hot_memories(store: &SqliteSessionStore, now: DateTime<Utc>) -> Result<usize> {
    let cutoff = (now - Duration::days(MIN_AGE_DAYS)).to_rfc3339();

    // Load all eligible hot memories upfront so we don't hold a prepared statement open.
    let candidates: Vec<HotCandidate> = {
        let connection = store.connect()?;
        let mut stmt = connection.prepare(
            "
            SELECT id, content, summary, tags, importance, COALESCE(memory_type, 'episodic')
            FROM hot_memories
            WHERE datetime(created_at) <= datetime(?1)
            ORDER BY datetime(created_at) ASC
            ",
        )?;

        stmt.query_map(params![cutoff], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, f32>(4)?,
                row.get::<_, String>(5)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()
        .context("failed to query hot memories for consolidation")?
        .into_iter()
        .map(|(id, content, summary, tags_raw, importance, mt_str)| {
            let tags = serde_json::from_str::<Vec<String>>(&tags_raw).unwrap_or_default();
            HotCandidate {
                id,
                content,
                summary,
                tags,
                importance,
                memory_type: MemoryType::parse(&mt_str),
            }
        })
        .collect()
    };

    // Build tag → candidate indices map.
    let mut tag_index: HashMap<String, Vec<usize>> = HashMap::new();
    for (idx, candidate) in candidates.iter().enumerate() {
        for tag in &candidate.tags {
            tag_index.entry(tag.clone()).or_default().push(idx);
        }
    }

    // Process tags in sorted order for determinism.
    let mut tags_sorted: Vec<String> = tag_index.keys().cloned().collect();
    tags_sorted.sort();

    let mut promoted: HashSet<i64> = HashSet::new();
    let mut clusters_created = 0usize;

    for tag in &tags_sorted {
        let indices = &tag_index[tag];
        let eligible: Vec<usize> = indices
            .iter()
            .copied()
            .filter(|&i| !promoted.contains(&candidates[i].id))
            .collect();

        if eligible.len() < MIN_CLUSTER_SIZE {
            continue;
        }

        let members: Vec<&HotCandidate> = eligible.iter().map(|&i| &candidates[i]).collect();
        let source_ids: Vec<i64> = members.iter().map(|m| m.id).collect();

        let dominant_type = dominant_memory_type(members.iter().map(|m| m.memory_type));
        let avg_importance =
            members.iter().map(|m| m.importance).sum::<f32>() / members.len() as f32;
        let cold_weight = avg_importance.clamp(0.5, 1.5);

        let mut all_tags: Vec<String> = members
            .iter()
            .flat_map(|m| m.tags.iter().cloned())
            .collect();
        all_tags.sort();
        all_tags.dedup();

        let content = build_consolidated_content(tag, &members);

        store
            .insert_cold_memory(NewColdMemory {
                content,
                weight: cold_weight,
                tags: all_tags,
                source_ids: source_ids.clone(),
                contradicts: Vec::new(),
                memory_type: dominant_type,
            })
            .context("failed to insert consolidated cold memory")?;

        // Delete the promoted hot memories.
        {
            let connection = store.connect()?;
            for &id in &source_ids {
                connection
                    .execute("DELETE FROM hot_fts WHERE rowid = ?1", params![id])
                    .ok();
                connection
                    .execute("DELETE FROM hot_memories WHERE id = ?1", params![id])
                    .with_context(|| format!("failed to delete consolidated hot memory {id}"))?;
            }
        }

        for &id in &source_ids {
            promoted.insert(id);
        }
        clusters_created += 1;
    }

    Ok(clusters_created)
}

/// Delete cold memories that have been at the weight floor long past their decay window.
fn prune_cold_memories(store: &SqliteSessionStore, now: DateTime<Utc>) -> Result<usize> {
    let candidates: Vec<(i64, String, String)> = {
        let connection = store.connect()?;
        let mut stmt = connection.prepare(
            "
            SELECT id, last_reinforced, COALESCE(memory_type, 'episodic')
            FROM cold_memories
            WHERE weight <= ?1 AND last_reinforced IS NOT NULL
            ",
        )?;

        stmt.query_map(params![COLD_MEMORY_FLOOR + 0.001_f32], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()
        .context("failed to query cold memories for pruning")?
    };

    let mut pruned = 0usize;
    for (id, last_reinforced, mt_str) in &candidates {
        let memory_type = MemoryType::parse(mt_str);
        let prune_threshold = now - Duration::days(memory_type.decay_days() * PRUNE_MULTIPLIER);

        let is_dead = DateTime::parse_from_rfc3339(last_reinforced)
            .map(|ts| ts.with_timezone(&Utc) <= prune_threshold)
            .unwrap_or(false);

        if is_dead {
            let connection = store.connect()?;
            connection
                .execute("DELETE FROM cold_fts WHERE rowid = ?1", params![id])
                .ok();
            connection
                .execute("DELETE FROM cold_memories WHERE id = ?1", params![id])
                .with_context(|| format!("failed to prune cold memory {id}"))?;
            pruned += 1;
        }
    }

    Ok(pruned)
}

fn dominant_memory_type(types: impl Iterator<Item = MemoryType>) -> MemoryType {
    let mut counts = [0usize; 3]; // [episodic, semantic, procedural]
    for t in types {
        match t {
            MemoryType::Episodic => counts[0] += 1,
            MemoryType::Semantic => counts[1] += 1,
            MemoryType::Procedural => counts[2] += 1,
        }
    }
    if counts[2] >= counts[1] && counts[2] >= counts[0] {
        MemoryType::Procedural
    } else if counts[1] >= counts[0] {
        MemoryType::Semantic
    } else {
        MemoryType::Episodic
    }
}

fn build_consolidated_content(tag: &str, members: &[&HotCandidate]) -> String {
    let lines: Vec<String> = members
        .iter()
        .map(|m| {
            m.summary
                .as_deref()
                .unwrap_or(m.content.as_str())
                .to_string()
        })
        .collect();
    format!(
        "Consolidated memory [{tag}] from {} sessions:\n{}",
        members.len(),
        lines.join("\n- ")
    )
}
