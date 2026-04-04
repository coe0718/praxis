use anyhow::{Context, Result};
use rusqlite::params;

use crate::{anatomy::NewAnatomyEntry, storage::AnatomyStore};

use super::SqliteSessionStore;

impl AnatomyStore for SqliteSessionStore {
    fn upsert_anatomy_entry(&self, entry: &NewAnatomyEntry) -> Result<()> {
        let connection = self.connect()?;
        let tags =
            serde_json::to_string(&entry.tags).context("failed to serialize anatomy tags")?;
        connection
            .execute(
                "
                INSERT INTO anatomy_index(path, description, token_estimate, last_modified_at, tags_json)
                VALUES (?1, ?2, ?3, ?4, ?5)
                ON CONFLICT(path) DO UPDATE SET
                    description = excluded.description,
                    token_estimate = excluded.token_estimate,
                    last_modified_at = excluded.last_modified_at,
                    tags_json = excluded.tags_json
                ",
                params![
                    entry.path,
                    entry.description,
                    entry.token_estimate,
                    entry.last_modified_at,
                    tags,
                ],
            )
            .context("failed to upsert anatomy entry")?;
        Ok(())
    }

    fn anatomy_entry_count(&self) -> Result<i64> {
        let connection = self.connect()?;
        connection
            .query_row("SELECT COUNT(*) FROM anatomy_index", [], |row| row.get(0))
            .context("failed to count anatomy entries")
    }
}
