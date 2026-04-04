use std::path::PathBuf;

use anyhow::Result;
use clap::{Args, Subcommand};

use crate::{
    learning::{OpportunityStatus, render_list, render_run, run_once},
    storage::{SessionStore, SqliteSessionStore},
    time::{Clock, SystemClock},
};

use super::core::load_initialized_config;

#[derive(Debug, Args)]
pub struct LearningArgs {
    #[command(subcommand)]
    command: LearningCommand,
}

#[derive(Debug, Subcommand)]
enum LearningCommand {
    Run,
    List,
}

pub(crate) fn handle_learning(
    data_dir_override: Option<PathBuf>,
    args: LearningArgs,
) -> Result<String> {
    let (_, paths) = load_initialized_config(data_dir_override)?;
    let store = SqliteSessionStore::new(paths.database_file.clone());
    store.initialize()?;
    store.validate_schema()?;

    match args.command {
        LearningCommand::Run => {
            let now = SystemClock::from_env()?.now_utc();
            Ok(render_run(&run_once(&paths, &store, now)?))
        }
        LearningCommand::List => Ok(render_list(
            store.latest_learning_run()?,
            &store.list_opportunities(OpportunityStatus::Pending, 10)?,
        )),
    }
}
