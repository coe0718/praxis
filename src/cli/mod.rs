mod agents;
pub(crate) mod approvals;
mod archive;
mod argus;
mod boundaries;
mod canary;
pub(crate) mod core;
mod forensics;
mod heartbeat;
mod learning;
mod oauth;
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
    Boundaries(boundaries::BoundariesArgs),
    Export(archive::ExportArgs),
    Import(archive::ImportArgs),
    Argus(argus::ArgusArgs),
    Canary(canary::CanaryArgs),
    Learn(learning::LearningArgs),
    Forensics(forensics::ForensicsArgs),
    Heartbeat(heartbeat::HeartbeatArgs),
    Queue(QueueArgs),
    Approve(ApprovalActionArgs),
    Reject(ApprovalActionArgs),
    Telegram(telegram::TelegramArgs),
    Serve(serve::ServeArgs),
    Tools(tools::ToolsArgs),
    Wake(WakeArgs),
    Bench(BenchArgs),
    Compact(CompactArgs),
    OAuth(oauth::OAuthArgs),
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
    #[arg(long = "file")]
    pub files: Vec<PathBuf>,

    #[arg(long, default_value = "reject")]
    pub attachment_policy: String,

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

#[derive(Debug, Args)]
pub struct WakeArgs {
    /// Human-readable reason for the wake request.
    pub reason: Vec<String>,

    /// Optional task to inject into the upcoming session.
    #[arg(long)]
    pub task: Option<String>,

    /// Source identifier (defaults to "cli").
    #[arg(long, default_value = "cli")]
    pub source: String,

    /// Mark the intent as urgent, bypassing quiet-hours.
    #[arg(long)]
    pub urgent: bool,
}

#[derive(Debug, Args)]
pub struct BenchArgs {
    /// Show previous results from the log instead of running benchmarks.
    #[arg(long)]
    pub log: bool,
}

#[derive(Debug, Args)]
pub struct CompactArgs {
    /// Goal currently in progress (included in the compaction record).
    #[arg(long)]
    pub goal: Option<String>,
}

pub fn run() -> Result<()> {
    let cli = Cli::parse();
    let output = execute(cli)?;
    println!("{output}");
    Ok(())
}

fn execute(cli: Cli) -> Result<String> {
    log::debug!("dispatching command: {:?}", cli.command);
    match cli.command {
        Commands::Init(args) => core::handle_init(cli.data_dir, args),
        Commands::Ask(args) => core::handle_ask(cli.data_dir, args),
        Commands::Run(args) => core::handle_run(cli.data_dir, args),
        Commands::Status => core::handle_status(cli.data_dir),
        Commands::Doctor => core::handle_doctor(cli.data_dir),
        Commands::Agents(args) => agents::handle_agents(cli.data_dir, args),
        Commands::Boundaries(args) => boundaries::handle_boundaries(cli.data_dir, args),
        Commands::Export(args) => archive::handle_export(cli.data_dir, args),
        Commands::Import(args) => archive::handle_import(cli.data_dir, args),
        Commands::Argus(args) => argus::handle_argus(cli.data_dir, args),
        Commands::Canary(args) => canary::handle_canary(cli.data_dir, args),
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
        Commands::Wake(args) => handle_wake(cli.data_dir, args),
        Commands::Bench(args) => handle_bench(cli.data_dir, args),
        Commands::Compact(args) => handle_compact(cli.data_dir, args),
        Commands::OAuth(args) => oauth::handle_oauth(cli.data_dir, args),
    }
}

fn handle_bench(data_dir_override: Option<PathBuf>, args: BenchArgs) -> Result<String> {
    use crate::{
        bench::{BenchmarkSuite, summarize_results},
        paths::{PraxisPaths, default_data_dir},
    };

    let data_dir = data_dir_override.unwrap_or(default_data_dir()?);
    let paths = PraxisPaths::for_data_dir(data_dir);

    if args.log {
        let log = BenchmarkSuite::load_log(&paths)?;
        if log.is_empty() {
            return Ok("No benchmark results recorded yet.".to_string());
        }
        let mut lines = Vec::new();
        for result in &log {
            lines.push(format!(
                "[{}] {} — {} ({})",
                result.ran_at.format("%Y-%m-%d %H:%M"),
                result.case_id,
                result.status_label(),
                result.summary,
            ));
        }
        return Ok(lines.join("\n"));
    }

    let count = BenchmarkSuite.validate(&paths)?;
    if count == 0 {
        return Ok(format!(
            "No benchmark cases found in {}",
            paths.benchmarks_dir.display()
        ));
    }

    let results = BenchmarkSuite.run(&paths)?;
    let summary = summarize_results(&results);
    let mut lines = vec![summary];
    for result in &results {
        lines.push(format!("  {} — {}", result.case_id, result.summary));
    }
    Ok(lines.join("\n"))
}

fn handle_compact(data_dir_override: Option<PathBuf>, args: CompactArgs) -> Result<String> {
    use crate::{
        context::CompactionRequest,
        context::request_compact,
        paths::{PraxisPaths, default_data_dir},
        time::Clock,
        time::SystemClock,
    };

    let data_dir = data_dir_override.unwrap_or(default_data_dir()?);
    let paths = PraxisPaths::for_data_dir(data_dir);
    let clock = SystemClock::from_env()?;
    let now = clock.now_utc();

    let req = CompactionRequest::operator(args.goal.clone(), now);
    request_compact(&paths.data_dir, &req)?;

    Ok(format!(
        "compaction requested\ntrigger: operator\ngoal: {}",
        args.goal.as_deref().unwrap_or("-")
    ))
}

fn handle_wake(data_dir_override: Option<PathBuf>, args: WakeArgs) -> Result<String> {
    use crate::{
        paths::{PraxisPaths, default_data_dir},
        wakeup::{WakeIntent, request_wake},
    };

    let data_dir = data_dir_override.unwrap_or(default_data_dir()?);
    let paths = PraxisPaths::for_data_dir(data_dir);

    let reason = args.reason.join(" ");
    if reason.trim().is_empty() {
        anyhow::bail!("reason is required");
    }

    let mut intent = WakeIntent::new(reason.trim(), &args.source);
    if let Some(task) = args.task {
        intent = intent.with_task(task);
    }
    if args.urgent {
        intent = intent.urgent();
    }

    request_wake(&paths.data_dir, &intent)?;

    Ok(format!(
        "wake intent written\nsource: {}\npriority: {}\nreason: {}",
        intent.source,
        if intent.is_urgent() {
            "urgent"
        } else {
            "normal"
        },
        intent.reason,
    ))
}
