mod agents;
pub(crate) mod approvals;
mod archive;
mod argus;
pub(crate) mod core;
mod forensics;
mod heartbeat;
mod learning;
mod serve;
mod telegram;
mod tools;

use std::path::PathBuf;

use anyhow::Result;
use clap::{Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "praxis")]
#[command(about = "Praxis foundation CLI")]
pub struct Cli {
    #[arg(long, global = true)]
    pub data_dir: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    Init(InitArgs),
    Ask(AskArgs),
    Run(RunArgs),
    Status,
    Doctor,
    Agents(agents::AgentsArgs),
    Export(archive::ExportArgs),
    Import(archive::ImportArgs),
    Argus(argus::ArgusArgs),
    Learn(learning::LearningArgs),
    Forensics(forensics::ForensicsArgs),
    Heartbeat(heartbeat::HeartbeatArgs),
    Queue(QueueArgs),
    Approve(ApprovalActionArgs),
    Reject(ApprovalActionArgs),
    Telegram(telegram::TelegramArgs),
    Serve(serve::ServeArgs),
    Tools(tools::ToolsArgs),
}

#[derive(Debug, Args)]
pub struct InitArgs {
    #[arg(long, default_value = "Praxis")]
    pub name: String,

    #[arg(long, default_value = "UTC")]
    pub timezone: String,

    #[arg(long, default_value_t = 2)]
    pub security_level: u8,
}

#[derive(Debug, Args)]
pub struct RunArgs {
    #[arg(long)]
    pub once: bool,

    #[arg(long)]
    pub force: bool,

    #[arg(long)]
    pub task: Option<String>,
}

#[derive(Debug, Args)]
pub struct AskArgs {
    #[arg(required = true)]
    pub prompt: Vec<String>,
}

#[derive(Debug, Args)]
pub struct QueueArgs {
    #[arg(long)]
    pub all: bool,
}

#[derive(Debug, Args)]
pub struct ApprovalActionArgs {
    pub id: i64,

    #[arg(long)]
    pub note: Option<String>,
}

pub fn run() -> Result<()> {
    let cli = Cli::parse();
    let output = execute(cli)?;
    println!("{output}");
    Ok(())
}

fn execute(cli: Cli) -> Result<String> {
    match cli.command {
        Commands::Init(args) => core::handle_init(cli.data_dir, args),
        Commands::Ask(args) => core::handle_ask(cli.data_dir, args),
        Commands::Run(args) => core::handle_run(cli.data_dir, args),
        Commands::Status => core::handle_status(cli.data_dir),
        Commands::Doctor => core::handle_doctor(cli.data_dir),
        Commands::Agents(args) => agents::handle_agents(cli.data_dir, args),
        Commands::Export(args) => archive::handle_export(cli.data_dir, args),
        Commands::Import(args) => archive::handle_import(cli.data_dir, args),
        Commands::Argus(args) => argus::handle_argus(cli.data_dir, args),
        Commands::Learn(args) => learning::handle_learning(cli.data_dir, args),
        Commands::Forensics(args) => forensics::handle_forensics(cli.data_dir, args),
        Commands::Heartbeat(args) => heartbeat::handle_heartbeat(cli.data_dir, args),
        Commands::Queue(args) => approvals::handle_queue(cli.data_dir, args),
        Commands::Approve(args) => approvals::handle_approval_action(
            cli.data_dir,
            args,
            crate::storage::ApprovalStatus::Approved,
        ),
        Commands::Reject(args) => approvals::handle_approval_action(
            cli.data_dir,
            args,
            crate::storage::ApprovalStatus::Rejected,
        ),
        Commands::Telegram(args) => telegram::handle_telegram(cli.data_dir, args),
        Commands::Serve(args) => serve::handle_serve(cli.data_dir, args),
        Commands::Tools(args) => tools::handle_tools(cli.data_dir, args),
    }
}
