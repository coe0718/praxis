use std::{fs, path::Path};

use anyhow::{Context, Result};

use crate::{
    anatomy::{build_entry, render_summary},
    state::{FileReadRecord, SessionState},
    storage::AnatomyStore,
};

#[derive(Debug, Default, Clone, Copy)]
pub struct TrackedContextReader;

impl TrackedContextReader {
    pub fn read<S: AnatomyStore>(
        &self,
        store: &S,
        state: &mut SessionState,
        path: &Path,
        reason: &str,
    ) -> Result<String> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let entry = build_entry(path, &content, vec![reason.to_string()])?;
        store.upsert_anatomy_entry(&entry)?;

        if already_read(state, &entry.path, &entry.last_modified_at) {
            state.repeated_reads_avoided += 1;
            return Ok(format!(
                "Repeated read avoided. {}",
                render_summary(&entry.path, &entry.description, entry.token_estimate)
            ));
        }

        state.file_reads.push(FileReadRecord {
            path: entry.path,
            modified_at: entry.last_modified_at,
            reason: reason.to_string(),
            token_estimate: entry.token_estimate,
        });
        Ok(content)
    }
}

fn already_read(state: &SessionState, path: &str, modified_at: &str) -> bool {
    state
        .file_reads
        .iter()
        .any(|read| read.path == path && read.modified_at == modified_at)
}
