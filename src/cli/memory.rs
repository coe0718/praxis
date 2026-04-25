use std::path::PathBuf;

use anyhow::Result;
use clap::{Args, Subcommand};

use crate::{
    memory::MemoryStore,
    storage::{SessionStore, SqliteSessionStore},
    time::{Clock, SystemClock},
};

use super::core::load_initialized_config;

#[derive(Debug, Args)]
pub struct MemoryArgs {
    #[command(subcommand)]
    command: MemoryCommand,
}

#[derive(Debug, Subcommand)]
enum MemoryCommand {
    /// Promote clustered hot memories to cold and prune dead cold memories.
    Consolidate,
}

pub(crate) fn handle_memory(
    data_dir_override: Option<PathBuf>,
    args: MemoryArgs,
) -> Result<String> {
    let (_, paths) = load_initialized_config(data_dir_override)?;
    let store = SqliteSessionStore::new(paths.database_file.clone());
    store.initialize()?;
    store.validate_schema()?;

    match args.command {
        MemoryCommand::Consolidate => {
            let now = SystemClock::from_env()?.now_utc();
            let summary = store.consolidate_memories(now)?;
            if summary.consolidated == 0 && summary.pruned == 0 {
                Ok("No consolidation needed — no qualifying clusters or dead memories.".to_string())
            } else {
                Ok(format!(
                    "Consolidated {} hot cluster(s) into cold memories. Pruned {} dead cold memory/memories.",
                    summary.consolidated, summary.pruned
                ))
            }
        }
    }
}
