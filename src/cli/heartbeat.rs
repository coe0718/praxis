use std::path::PathBuf;

use anyhow::Result;
use clap::{Args, Subcommand};

use crate::{
    cli::core::load_initialized_config,
    heartbeat::{HeartbeatRecord, check_heartbeat, read_heartbeat},
    time::SystemClock,
};

#[derive(Debug, Args)]
pub struct HeartbeatArgs {
    #[command(subcommand)]
    pub command: HeartbeatCommand,
}

#[derive(Debug, Subcommand)]
pub enum HeartbeatCommand {
    Status,
    Check(HeartbeatCheckArgs),
}

#[derive(Debug, Args)]
pub struct HeartbeatCheckArgs {
    #[arg(long, default_value_t = 900)]
    pub max_age_seconds: i64,
}

pub(crate) fn handle_heartbeat(
    data_dir_override: Option<PathBuf>,
    args: HeartbeatArgs,
) -> Result<String> {
    let (_, paths) = load_initialized_config(data_dir_override)?;
    match args.command {
        HeartbeatCommand::Status => {
            let record = read_heartbeat(&paths.heartbeat_file)?;
            Ok(render_record("heartbeat", &record))
        }
        HeartbeatCommand::Check(args) => {
            let clock = SystemClock::from_env()?;
            let record = check_heartbeat(&clock, &paths.heartbeat_file, args.max_age_seconds)?;
            Ok(render_record("heartbeat_check", &record))
        }
    }
}

fn render_record(kind: &str, record: &HeartbeatRecord) -> String {
    format!(
        "{kind}: ok\ncomponent: {}\nphase: {}\nupdated_at: {}\npid: {}\nprocess_uptime_ms: {}\ndetail: {}",
        record.component,
        record.phase,
        record.updated_at,
        record.pid,
        record.process_uptime_ms,
        record.detail
    )
}
