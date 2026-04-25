use anyhow::{Context, Result, bail};
use rusqlite::params;

use crate::memory::{MemoryLink, MemoryLinkStore, MemoryLinkType, MemoryStore, StoredMemory};

use super::SqliteSessionStore;

/// Internal helper for the conflict workbench.
pub(crate) trait ContradictionQuery {
    fn all_contradiction_pairs(&self) -> Result<Vec<(i64, String, i64, String)>>;
}

impl ContradictionQuery for SqliteSessionStore {
    fn all_contradiction_pairs(&self) -> Result<Vec<(i64, String, i64, String)>> {
        let connection = self.connect()?;
        let mut statement = connection.prepare(
            "SELECT from_memory_id, to_memory_id FROM memory_links WHERE link_type = 'contradicts'",
        )?;
        let pairs: Vec<(i64, i64)> = statement
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<rusqlite::Result<_>>()
            .context("failed to query contradiction links")?;

        let mut result = Vec::new();
        for (from_id, to_id) in pairs {
            let from_text = fetch_memory_content(&connection, from_id)?;
            let to_text = fetch_memory_content(&connection, to_id)?;
            if let (Some(f), Some(t)) = (from_text, to_text) {
                result.push((from_id, f, to_id, t));
            }
        }
        Ok(result)
    }
}

impl MemoryLinkStore for SqliteSessionStore {
    fn add_memory_link(&self, from_id: i64, to_id: i64, link_type: MemoryLinkType) -> Result<()> {
        if from_id == to_id {
            bail!("a memory cannot link to itself");
        }
        if self.get_memory(from_id)?.is_none() {
            bail!("memory {from_id} does not exist");
        }
        if self.get_memory(to_id)?.is_none() {
            bail!("memory {to_id} does not exist");
        }

        let connection = self.connect()?;
        connection
            .execute(
                "INSERT OR IGNORE INTO memory_links(from_memory_id, to_memory_id, link_type)
                 VALUES (?1, ?2, ?3)",
                params![from_id, to_id, link_type.as_str()],
            )
            .context("failed to insert memory link")?;

        Ok(())
    }

    fn links_for(&self, memory_id: i64) -> Result<Vec<MemoryLink>> {
        let connection = self.connect()?;
        let mut statement = connection.prepare(
            "SELECT id, from_memory_id, to_memory_id, link_type
             FROM memory_links
             WHERE from_memory_id = ?1 OR to_memory_id = ?1
             ORDER BY id",
        )?;

        let rows = statement.query_map(params![memory_id], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, String>(3)?,
            ))
        })?;

        let mut links = Vec::new();
        for row in rows {
            let (id, from_id, to_id, link_type_str) = row.context("failed to read memory link")?;
            let link_type = MemoryLinkType::parse(&link_type_str)
                .ok_or_else(|| anyhow::anyhow!("unknown link type: {link_type_str}"))?;
            links.push(MemoryLink {
                id,
                from_memory_id: from_id,
                to_memory_id: to_id,
                link_type,
            });
        }

        Ok(links)
    }

    fn linked_memories(&self, memory_id: i64, limit: usize) -> Result<Vec<StoredMemory>> {
        let links = self.links_for(memory_id)?;
        let mut memories = Vec::new();

        for link in links.iter().take(limit) {
            let other_id = if link.from_memory_id == memory_id {
                link.to_memory_id
            } else {
                link.from_memory_id
            };
            if let Some(memory) = self.get_memory(other_id)? {
                memories.push(memory);
            }
        }

        Ok(memories)
    }
}

fn fetch_memory_content(connection: &rusqlite::Connection, id: i64) -> Result<Option<String>> {
    use rusqlite::OptionalExtension;
    let hot: Option<String> = connection
        .query_row("SELECT content FROM hot_memories WHERE id = ?1", params![id], |row| row.get(0))
        .optional()
        .context("failed to fetch hot memory content")?;
    if hot.is_some() {
        return Ok(hot);
    }
    let cold: Option<String> = connection
        .query_row("SELECT content FROM cold_memories WHERE id = ?1", params![id], |row| row.get(0))
        .optional()
        .context("failed to fetch cold memory content")?;
    Ok(cold)
}
