use std::path::PathBuf;

use anyhow::Result;
use clap::{Args, Subcommand};

use crate::{
    canary::{CanaryFreezeState, ModelCanaryLedger, run_canaries},
    cli::core::load_initialized_config,
};

#[derive(Debug, Args)]
pub struct CanaryArgs {
    #[command(subcommand)]
    pub command: CanaryCommand,
}

#[derive(Debug, Subcommand)]
pub enum CanaryCommand {
    Run(CanaryRunArgs),
    Status,
}

#[derive(Debug, Args)]
pub struct CanaryRunArgs {
    #[arg(long)]
    pub provider: Option<String>,
}

pub(crate) fn handle_canary(
    data_dir_override: Option<PathBuf>,
    args: CanaryArgs,
) -> Result<String> {
    match args.command {
        CanaryCommand::Run(args) => run(data_dir_override, args),
        CanaryCommand::Status => status(data_dir_override),
    }
}

fn run(data_dir_override: Option<PathBuf>, args: CanaryRunArgs) -> Result<String> {
    let (config, paths) = load_initialized_config(data_dir_override)?;
    let records = run_canaries(&config, &paths, args.provider.as_deref())?;
    let mut lines = vec![format!("canary_runs: {}", records.len())];
    for record in records {
        lines.push(format!(
            "{} {} {} eval_failures={} {}",
            record.provider,
            record.model,
            label(record.status),
            record.eval_failures,
            record.checked_at
        ));
    }
    Ok(lines.join("\n"))
}

fn status(data_dir_override: Option<PathBuf>) -> Result<String> {
    let (config, paths) = load_initialized_config(data_dir_override)?;
    let ledger = ModelCanaryLedger::load_or_default(&paths.model_canary_file)?;
    let freeze = CanaryFreezeState::load_or_default(&paths.canary_freeze_file)?;
    let mut lines = vec![format!(
        "freeze_on_model_regression: {}",
        config.agent.freeze_on_model_regression
    )];
    if !freeze.frozen.is_empty() {
        lines.push(format!(
            "frozen: {}",
            freeze.frozen.into_iter().collect::<Vec<_>>().join(", ")
        ));
    }
    if ledger.records.is_empty() {
        lines.push("canaries: none recorded".to_string());
        return Ok(lines.join("\n"));
    }
    for record in ledger.records {
        lines.push(format!(
            "{} {} {} eval_failures={} consecutive_passes={} {}",
            record.provider,
            record.model,
            label(record.status),
            record.eval_failures,
            record.consecutive_passes,
            record.checked_at
        ));
    }
    Ok(lines.join("\n"))
}

fn label(status: crate::canary::CanaryStatus) -> &'static str {
    match status {
        crate::canary::CanaryStatus::Passed => "passed",
        crate::canary::CanaryStatus::Failed => "failed",
    }
}
