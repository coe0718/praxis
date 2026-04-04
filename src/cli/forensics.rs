use std::{fmt::Write as _, path::PathBuf};

use anyhow::{Context, Result, bail};
use clap::{Args, Subcommand};

use crate::forensics::{latest_started_at, load_snapshots};

use super::core::load_initialized_config;

#[derive(Debug, Args)]
pub struct ForensicsArgs {
    #[command(subcommand)]
    command: ForensicsCommand,
}

#[derive(Debug, Subcommand)]
enum ForensicsCommand {
    Latest,
    Session(ForensicsSessionArgs),
}

#[derive(Debug, Args)]
struct ForensicsSessionArgs {
    #[arg(long)]
    started_at: String,
}

pub(crate) fn handle_forensics(
    data_dir_override: Option<PathBuf>,
    args: ForensicsArgs,
) -> Result<String> {
    match args.command {
        ForensicsCommand::Latest => render_latest(data_dir_override),
        ForensicsCommand::Session(args) => render_session(data_dir_override, &args.started_at),
    }
}

fn render_latest(data_dir_override: Option<PathBuf>) -> Result<String> {
    let (_, paths) = load_initialized_config(data_dir_override)?;
    let started_at = latest_started_at(&paths.database_file)?.with_context(|| {
        format!(
            "no session snapshots found in {}",
            paths.database_file.display()
        )
    })?;
    render_snapshots(&paths.database_file, &started_at)
}

fn render_session(data_dir_override: Option<PathBuf>, started_at: &str) -> Result<String> {
    let (_, paths) = load_initialized_config(data_dir_override)?;
    render_snapshots(&paths.database_file, started_at)
}

fn render_snapshots(database_file: &std::path::Path, started_at: &str) -> Result<String> {
    let snapshots = load_snapshots(database_file, started_at)?;
    if snapshots.is_empty() {
        bail!("no snapshots found for session {started_at}");
    }

    let mut output = String::new();
    writeln!(output, "session_started_at: {started_at}")?;
    writeln!(output, "snapshot_count: {}", snapshots.len())?;
    if let Some(session_id) = snapshots.iter().find_map(|snapshot| snapshot.session_id) {
        writeln!(output, "session_id: {session_id}")?;
    }

    for snapshot in snapshots {
        writeln!(
            output,
            "- {} {} phase={} outcome={}",
            snapshot.recorded_at,
            snapshot.checkpoint,
            snapshot.phase,
            snapshot.state.last_outcome.as_deref().unwrap_or("none")
        )?;
    }

    Ok(output.trim_end().to_string())
}
