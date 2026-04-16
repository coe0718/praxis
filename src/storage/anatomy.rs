use std::path::Path;

use anyhow::Result;

use crate::anatomy::NewAnatomyEntry;

pub trait AnatomyStore {
    fn upsert_anatomy_entry(&self, entry: &NewAnatomyEntry) -> Result<()>;
    fn anatomy_entry_count(&self) -> Result<i64>;
    /// Return the stored `last_modified_at` timestamp for the given path, or
    /// `None` if the path has never been indexed.
    fn anatomy_last_modified(&self, path: &Path) -> Result<Option<String>>;
}
