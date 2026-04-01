mod approvals;
mod core;
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
    Run(RunArgs),
    Status,
    Doctor,
    Queue(QueueArgs),
    Approve(ApprovalActionArgs),
    Reject(ApprovalActionArgs),
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
        Commands::Run(args) => core::handle_run(cli.data_dir, args),
        Commands::Status => core::handle_status(cli.data_dir),
        Commands::Doctor => core::handle_doctor(cli.data_dir),
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
        Commands::Tools(args) => tools::handle_tools(cli.data_dir, args),
    }
}
