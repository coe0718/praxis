//! Conflict workbench — surfaces contradicting memory pairs for operator review.
//!
//! Any two memories linked with `contradicts` are written into
//! `MEMORY_CONFLICTS.md` so the operator can resolve or dismiss them.
//! The file is regenerated in full each time so it stays authoritative.

use std::fs;

use anyhow::Result;

use crate::{
    paths::PraxisPaths,
    storage::{ContradictionQuery, SqliteSessionStore},
};

/// Regenerate `MEMORY_CONFLICTS.md` from all current `contradicts` links.
///
/// Returns the number of conflicting pairs written.
pub fn generate_conflict_workbench(paths: &PraxisPaths) -> Result<usize> {
    let store = SqliteSessionStore::new(paths.database_file.clone());
    let pairs = store.all_contradiction_pairs()?;

    if pairs.is_empty() {
        fs::write(
            &paths.memory_conflicts_file,
            "# Memory Conflicts\n\nNo contradictions recorded.\n",
        )?;
        return Ok(0);
    }

    let mut out = "# Memory Conflicts\n\n\
        This file is auto-generated. Resolve conflicts by removing the `contradicts` \
        link via Telegram `/link` or `/forget` commands.\n\n"
        .to_string();

    for (idx, (from_id, from_text, to_id, to_text)) in pairs.iter().enumerate() {
        out.push_str(&format!(
            "## Conflict {} — [{from_id}] vs [{to_id}]\n\n\
             **Memory [{from_id}]:**\n> {}\n\n\
             **Memory [{to_id}]:**\n> {}\n\n---\n\n",
            idx + 1,
            from_text.replace('\n', "\n> "),
            to_text.replace('\n', "\n> "),
        ));
    }

    fs::write(&paths.memory_conflicts_file, out)?;
    Ok(pairs.len())
}
