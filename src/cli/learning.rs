use std::path::PathBuf;

use anyhow::{Result, bail};
use clap::{Args, Subcommand};

use crate::{
    learning::{
        OpportunityStatus, append_note, render_action, render_list, render_note, render_run,
        run_once, update_opportunity,
    },
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
    Note(LearningNoteArgs),
    List(LearningListArgs),
    Accept(LearningActionArgs),
    Dismiss(LearningActionArgs),
}

#[derive(Debug, Args)]
struct LearningNoteArgs {
    #[arg(required = true, allow_hyphen_values = true)]
    text: Vec<String>,
}

#[derive(Debug, Args)]
struct LearningListArgs {
    #[arg(long)]
    all: bool,
}

#[derive(Debug, Args)]
struct LearningActionArgs {
    id: i64,
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
        LearningCommand::Note(args) => {
            let now = SystemClock::from_env()?.now_utc();
            let note = args.text.join(" ");
            Ok(render_note(&append_note(&paths, &note, now)?))
        }
        LearningCommand::List(args) => Ok(render_list(
            store.latest_learning_run()?,
            &store.list_opportunities(OpportunityStatus::Pending, 10)?,
            &store.list_opportunities(OpportunityStatus::Accepted, 10)?,
            &store.list_opportunities(OpportunityStatus::Dismissed, 10)?,
            args.all,
        )),
        LearningCommand::Accept(args) => {
            let now = SystemClock::from_env()?.now_utc();
            let Some(opportunity) =
                update_opportunity(&paths, &store, args.id, OpportunityStatus::Accepted, now)?
            else {
                bail!("opportunity {} was not found", args.id);
            };
            Ok(render_action("accept", &opportunity))
        }
        LearningCommand::Dismiss(args) => {
            let now = SystemClock::from_env()?.now_utc();
            let Some(opportunity) =
                update_opportunity(&paths, &store, args.id, OpportunityStatus::Dismissed, now)?
            else {
                bail!("opportunity {} was not found", args.id);
            };
            Ok(render_action("dismiss", &opportunity))
        }
    }
}
