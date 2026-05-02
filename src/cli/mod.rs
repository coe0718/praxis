mod acp;
mod agents;
pub(crate) mod approvals;
mod archive;
mod argus;
mod boundaries;
mod brief;
pub(crate) mod canary;
mod chat;
mod checkpoint;
pub(crate) mod core;
mod daemon;
mod delegation;
#[cfg(feature = "discord")]
mod discord;
mod dryrun;
mod evolution;
mod forensics;
pub(crate) mod git;
mod hands;
mod heartbeat;
mod hooks;
mod learning;
mod mcp;
mod memory;
mod migrate;
mod oauth;
mod profile;
mod sandbox;
mod serve;
#[cfg(feature = "slack")]
mod slack;
mod telegram;
mod tools;
#[cfg(feature = "tui")]
mod tui;
mod vault;
mod vscode;
mod watchdog;
mod webhook;
pub mod wizard;
mod worktree;

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
    Mcp(mcp::McpArgs),
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
    Git(git::GitArgs),
    Watchdog(watchdog::WatchdogArgs),
    #[cfg(feature = "discord")]
    Discord(discord::DiscordArgs),
    #[cfg(feature = "slack")]
    Slack(slack::SlackArgs),
    Vscode(vscode::VscodeArgs),
    #[cfg(feature = "tui")]
    Tui(tui::TuiArgs),
    Brief(brief::BriefArgs),
    Daemon(daemon::DaemonArgs),
    Delegation(delegation::DelegationArgs),
    Evolve(evolution::EvolveArgs),
    Hands(hands::HandsArgs),
    Hooks(hooks::HooksArgs),
    Sandbox(sandbox::SandboxArgs),
    Vault(vault::VaultArgs),
    Webhook(webhook::WebhookArgs),
    Memory(memory::MemoryArgs),
    Completions(CompletionsArgs),
    Sessions(SessionsArgs),
    Insights(InsightsArgs),
    Chat(ChatArgs),
    Acp,
    Checkpoint(CheckpointArgs),
    Rollback(RollbackArgs),
    Checkpoints,
    Migrate(MigrateArgs),
    Worktree(WorktreeArgs),
    Plan(PlanArgs),
    Profile(ProfileArgs),
}

#[derive(Debug, Args)]
pub struct CompletionsArgs {
    /// Shell to generate completions for (bash, zsh, fish, elvish, powershell).
    pub shell: String,
}

#[derive(Debug, Args)]
pub struct SessionsArgs {
    /// Search query for session outcomes and action summaries.
    pub query: Vec<String>,

    /// Maximum results to return (default 20).
    #[arg(long, default_value_t = 20)]
    pub limit: usize,
}

#[derive(Debug, Args)]
pub struct InsightsArgs {
    /// Number of days to look back (default 30).
    #[arg(long, default_value_t = 30)]
    pub days: u32,
}

#[derive(Debug, Args)]
pub struct ChatArgs {
    /// Override the model for this session.
    #[arg(long)]
    pub model: Option<String>,
}

#[derive(Debug, Args)]
pub struct CheckpointArgs {
    /// Label for this checkpoint.
    #[arg(long, default_value = "manual")]
    pub label: String,
}

#[derive(Debug, Args)]
pub struct RollbackArgs {
    /// Checkpoint ID to roll back to.
    pub id: u64,
}

#[derive(Debug, Args)]
pub struct MigrateArgs {
    /// Path to the Axonix data directory to import from.
    pub source: PathBuf,

    /// Show what would be imported without making changes.
    #[arg(long)]
    pub dry_run: bool,
}

#[derive(Debug, Args)]
pub struct WorktreeArgs {
    #[command(subcommand)]
    pub command: WorktreeCommand,
}

#[derive(Debug, Subcommand)]
pub enum WorktreeCommand {
    /// Create a new isolated worktree.
    Create(WorktreeCreateArgs),
    /// List all praxis worktrees.
    List,
    /// Remove a worktree.
    Remove(WorktreeRemoveArgs),
    /// Merge a worktree branch into target.
    Merge(WorktreeMergeArgs),
}

#[derive(Debug, Args)]
pub struct WorktreeCreateArgs {
    /// Name for the worktree (used as branch suffix).
    pub name: String,
}

#[derive(Debug, Args)]
pub struct WorktreeRemoveArgs {
    /// Name of the worktree to remove.
    pub name: String,

    /// Also delete the associated branch.
    #[arg(long)]
    pub delete_branch: bool,
}

#[derive(Debug, Args)]
pub struct WorktreeMergeArgs {
    /// Name of the worktree to merge.
    pub name: String,

    /// Target branch to merge into (default: main).
    #[arg(long, default_value = "main")]
    pub target_branch: String,
}

#[derive(Debug, Args)]
pub struct PlanArgs {
    #[command(subcommand)]
    pub command: PlanCommand,
}

#[derive(Debug, Subcommand)]
pub enum PlanCommand {
    /// Create a new execution plan.
    Create(PlanCreateArgs),
    /// List all plans.
    List,
    /// Show plan details.
    Show(PlanShowArgs),
    /// Dry-run validate a plan.
    DryRun(PlanDryRunArgs),
    /// Remove a plan.
    Remove(PlanRemoveArgs),
}

#[derive(Debug, Args)]
pub struct PlanCreateArgs {
    /// Plan ID.
    pub id: String,
    /// Human-readable label.
    #[arg(long, default_value = "")]
    pub label: String,
    /// Steps in "tool:param1=val1,param2=val2" format.
    #[arg(long)]
    pub step: Vec<String>,
}

#[derive(Debug, Args)]
pub struct PlanShowArgs {
    pub plan_id: String,
}

#[derive(Debug, Args)]
pub struct PlanDryRunArgs {
    pub plan_id: String,
}

#[derive(Debug, Args)]
pub struct PlanRemoveArgs {
    pub plan_id: String,
}

#[derive(Debug, Args)]
pub struct ProfileArgs {
    #[command(subcommand)]
    pub command: ProfileCommand,
}

#[derive(Debug, Subcommand)]
pub enum ProfileCommand {
    /// List all profiles.
    List,
    /// Create a new profile.
    Create(ProfileCreateArgs),
    /// Switch to a profile.
    Switch(ProfileSwitchArgs),
    /// Remove a profile.
    Remove(ProfileRemoveArgs),
    /// Show profile details.
    Show(ProfileShowArgs),
}

#[derive(Debug, Args)]
pub struct ProfileCreateArgs {
    pub name: String,
    #[arg(long, default_value = "")]
    pub description: String,
}

#[derive(Debug, Args)]
pub struct ProfileSwitchArgs {
    pub name: String,
}

#[derive(Debug, Args)]
pub struct ProfileRemoveArgs {
    pub name: String,
}

#[derive(Debug, Args)]
pub struct ProfileShowArgs {
    pub name: String,
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

    /// Override the execution profile (quality, budget, offline, deterministic).
    #[arg(long)]
    pub profile: Option<String>,
}

#[derive(Debug, Args)]
pub struct AskArgs {
    #[arg(long = "file")]
    pub files: Vec<PathBuf>,

    #[arg(long, default_value = "reject")]
    pub attachment_policy: String,

    /// Run the full agent loop with tool execution instead of a single LLM call.
    #[arg(long)]
    pub tools: bool,

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
        Commands::Mcp(args) => mcp::handle_mcp(cli.data_dir, args),
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
        Commands::Git(args) => git::handle_git(cli.data_dir, args),
        Commands::Watchdog(args) => watchdog::handle_watchdog(cli.data_dir, args),
        #[cfg(feature = "discord")]
        Commands::Discord(args) => discord::handle_discord(cli.data_dir, args),
        #[cfg(feature = "slack")]
        Commands::Slack(args) => slack::handle_slack(cli.data_dir, args),
        Commands::Vscode(args) => vscode::handle_vscode(cli.data_dir, args),
        #[cfg(feature = "tui")]
        Commands::Tui(args) => tui::handle_tui(cli.data_dir, args),
        Commands::Brief(args) => brief::handle_brief(cli.data_dir, args),
        Commands::Daemon(args) => daemon::handle_daemon(cli.data_dir, args),
        Commands::Delegation(args) => delegation::handle_delegation(cli.data_dir, args),
        Commands::Evolve(args) => evolution::handle_evolve(cli.data_dir, args),
        Commands::Hands(args) => hands::handle_hands(cli.data_dir, args),
        Commands::Hooks(args) => hooks::handle_hooks(cli.data_dir, args),
        Commands::Sandbox(args) => sandbox::handle_sandbox(cli.data_dir, args),
        Commands::Vault(args) => vault::handle_vault(cli.data_dir, args),
        Commands::Webhook(args) => webhook::handle_webhook(cli.data_dir, &args),
        Commands::Memory(args) => memory::handle_memory(cli.data_dir, args),
        Commands::Completions(args) => handle_completions(args),
        Commands::Sessions(args) => handle_sessions(cli.data_dir, args),
        Commands::Insights(args) => handle_insights(cli.data_dir, args),
        Commands::Chat(args) => {
            chat::run_interactive(cli.data_dir, args.model)?;
            Ok("Session ended.".to_string())
        }
        Commands::Acp => {
            acp::run_acp_server(cli.data_dir)?;
            Ok("ACP server stopped.".to_string())
        }
        Commands::Checkpoint(args) => checkpoint::handle_checkpoint(cli.data_dir, Some(args.label)),
        Commands::Rollback(args) => checkpoint::handle_rollback(cli.data_dir, args.id),
        Commands::Checkpoints => checkpoint::handle_checkpoints_list(cli.data_dir),
        Commands::Migrate(args) => migrate::handle_migrate(cli.data_dir, args.source, args.dry_run),
        Commands::Worktree(args) => worktree::handle_worktree(cli.data_dir, args),
        Commands::Plan(args) => dryrun::handle_plan(cli.data_dir, args),
        Commands::Profile(args) => profile::handle_profile(cli.data_dir, args),
    }
}

fn handle_completions(args: CompletionsArgs) -> Result<String> {
    use anyhow::Context;
    use clap::CommandFactory;
    use clap_complete::{Shell, generate};

    let mut cmd = Cli::command();
    let bin_name = cmd.get_name().to_string();

    let shell: Shell = match args.shell.as_str() {
        "bash" => Shell::Bash,
        "zsh" => Shell::Zsh,
        "fish" => Shell::Fish,
        "elvish" => Shell::Elvish,
        "powershell" => Shell::PowerShell,
        other => anyhow::bail!(
            "unsupported shell '{}'. Supported: bash, zsh, fish, elvish, powershell",
            other
        ),
    };

    let mut buf = Vec::new();
    generate(shell, &mut cmd, &bin_name, &mut buf);
    String::from_utf8(buf).context("generated completions contain invalid UTF-8")
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
        return Ok(format!("No benchmark cases found in {}", paths.benchmarks_dir.display()));
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
        if intent.is_urgent() { "urgent" } else { "normal" },
        intent.reason,
    ))
}

fn handle_sessions(data_dir_override: Option<PathBuf>, args: SessionsArgs) -> Result<String> {
    use crate::{
        paths::{PraxisPaths, default_data_dir},
        storage::SqliteSessionStore,
    };

    let data_dir = data_dir_override.unwrap_or(default_data_dir()?);
    let paths = PraxisPaths::for_data_dir(data_dir);

    if !paths.config_file.exists() {
        anyhow::bail!("Praxis is not initialized. Run `praxis init` first.");
    }

    let query = args.query.join(" ");
    if query.trim().is_empty() {
        anyhow::bail!("sessions: search query is required");
    }

    let store = SqliteSessionStore::new(paths.database_file.clone());
    let results = store.search_sessions(&query, args.limit)?;

    if results.is_empty() {
        return Ok(format!("sessions: no matches for \"{}\"", query));
    }

    let mut lines = vec![format!("sessions: {} match(es) for \"{}\":", results.len(), query)];
    for result in &results {
        let s = &result.session;
        lines.push(format!(
            "  [#{} session={}] {} ({}): {}",
            s.id, s.session_num, s.ended_at, result.matched_column, result.snippet,
        ));
    }
    Ok(lines.join("\n"))
}

fn handle_insights(data_dir_override: Option<PathBuf>, _args: InsightsArgs) -> Result<String> {
    use crate::{
        paths::{PraxisPaths, default_data_dir},
        storage::{SessionStore, SqliteSessionStore},
    };

    let data_dir = data_dir_override.unwrap_or(default_data_dir()?);
    let paths = PraxisPaths::for_data_dir(data_dir);

    if !paths.config_file.exists() {
        anyhow::bail!("Praxis is not initialized. Run `praxis init` first.");
    }

    let store = SqliteSessionStore::new(paths.database_file.clone());

    // Gather data
    let all_time = store.token_summary_all_time().ok();
    let sessions = store.token_usage_by_session(50).ok();
    let providers = store.token_usage_by_provider().ok();
    let hot = store.count_hot_memories().unwrap_or(0);
    let cold = store.count_cold_memories().unwrap_or(0);
    let pending = store.count_pending_approvals().unwrap_or(0);
    let last = store.last_session().ok().flatten();

    let mut lines = Vec::new();
    lines.push("insights:".to_string());

    // Session count
    if let Some(ref sessions) = sessions {
        lines.push(format!("  sessions (recent): {}", sessions.len()));
    }

    // Token usage — all time
    if let Some(ref at) = all_time {
        let k_tokens = at.total_tokens as f64 / 1000.0;
        lines.push(format!(
            "  tokens: {:.1}K total across {} sessions",
            k_tokens, at.total_sessions,
        ));
        if at.total_cost_micros > 0 {
            let cost = at.total_cost_micros as f64 / 1_000_000.0;
            lines.push(format!("  est. cost: ${:.2}", cost));
        }
    }

    // Top provider
    if let Some(ref providers) = providers
        && let Some(top) = providers.first()
    {
        lines.push(format!(
            "  top provider: {} ({:.1}K tokens)",
            top.provider,
            top.tokens_used as f64 / 1000.0,
        ));
    }

    // Memory
    lines.push(format!("  memory: {hot} hot / {cold} cold"));
    lines.push(format!("  pending approvals: {pending}"));

    // Latest session
    if let Some(ref last) = last {
        lines.push(format!(
            "  last session: #{} (session {}) at {} — {}",
            last.id, last.session_num, last.ended_at, last.outcome,
        ));
    }

    Ok(lines.join("\n"))
}
