use std::path::PathBuf;

use anyhow::Result;
use clap::{Args, Subcommand};

use crate::{
    archive::{export_audit, export_bundle, import_bundle},
    cli::core::load_initialized_config,
    paths::default_data_dir,
    time::{Clock, SystemClock},
};

#[derive(Debug, Args)]
pub struct ExportArgs {
    #[command(subcommand)]
    pub command: ExportCommand,
}

#[derive(Debug, Subcommand)]
pub enum ExportCommand {
    State(StateExportArgs),
    Audit(AuditExportArgs),
}

#[derive(Debug, Args)]
pub struct StateExportArgs {
    #[arg(long)]
    pub output: PathBuf,

    #[arg(long)]
    pub overwrite: bool,
}

#[derive(Debug, Args)]
pub struct AuditExportArgs {
    #[arg(long)]
    pub output: PathBuf,

    #[arg(long, default_value_t = 7)]
    pub days: i64,

    #[arg(long)]
    pub overwrite: bool,
}

#[derive(Debug, Args)]
pub struct ImportArgs {
    #[arg(long)]
    pub input: PathBuf,

    #[arg(long)]
    pub overwrite: bool,
}

pub(crate) fn handle_export(
    data_dir_override: Option<PathBuf>,
    args: ExportArgs,
) -> Result<String> {
    match args.command {
        ExportCommand::State(args) => export_state(data_dir_override, args),
        ExportCommand::Audit(args) => export_audit_report(data_dir_override, args),
    }
}

pub(crate) fn handle_import(
    data_dir_override: Option<PathBuf>,
    args: ImportArgs,
) -> Result<String> {
    let data_dir = data_dir_override.unwrap_or(default_data_dir()?);
    let summary = import_bundle(&data_dir, &args.input, args.overwrite)?;
    Ok(format!(
        "import: ok\ndata_dir: {}\nrestored_files: {}\nschema_version: {}",
        summary.data_dir.display(),
        summary.restored_file_count,
        summary.schema_version,
    ))
}

fn export_state(data_dir_override: Option<PathBuf>, args: StateExportArgs) -> Result<String> {
    let clock = SystemClock::from_env()?;
    let now = clock.now_utc();
    let (config, paths) = load_initialized_config(data_dir_override)?;
    let summary = export_bundle(&config, &paths, &args.output, args.overwrite, now)?;
    Ok(format!(
        "export: ok\nkind: state\noutput: {}\ndata_files: {}\nschema_version: {}",
        summary.output_dir.display(),
        summary.data_file_count,
        summary.schema_version,
    ))
}

fn export_audit_report(
    data_dir_override: Option<PathBuf>,
    args: AuditExportArgs,
) -> Result<String> {
    let clock = SystemClock::from_env()?;
    let now = clock.now_utc();
    let (_, paths) = load_initialized_config(data_dir_override)?;
    let summary = export_audit(&paths, &args.output, args.overwrite, now, args.days)?;
    Ok(format!(
        "export: ok\nkind: audit\noutput: {}\nsessions: {}\napprovals: {}",
        summary.output_file.display(),
        summary.session_count,
        summary.approval_count,
    ))
}
