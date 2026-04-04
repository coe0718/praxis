use anyhow::Result;

use crate::anatomy::NewAnatomyEntry;

pub trait AnatomyStore {
    fn upsert_anatomy_entry(&self, entry: &NewAnatomyEntry) -> Result<()>;
    fn anatomy_entry_count(&self) -> Result<i64>;
}
