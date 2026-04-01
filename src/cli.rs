use std::{fmt::Write as _, path::PathBuf};

use anyhow::{Context, Result, bail};
use clap::{Args, Parser, Subcommand};

use crate::{
    config::AppConfig,
    events::NoopEventSink,
    identity::{IdentityPolicy, LocalIdentityPolicy, MarkdownGoalParser},
    r#loop::{PraxisRuntime, RunOptions, StubBackend},
    paths::{PraxisPaths, default_data_dir},
    state::{SessionPhase, SessionState},
    storage::{SessionStore, SqliteSessionStore},
    time::{Clock, SystemClock},
};

#[derive(Debug, Parser)]
#[command(name = "praxis")]
#[command(about = "Praxis foundation CLI")]
pub struct Cli {
    #[arg(long, global = true)]
    data_dir: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Init(InitArgs),
    Run(RunArgs),
    Status,
    Doctor,
}

#[derive(Debug, Args)]
struct InitArgs {
    #[arg(long, default_value = "Praxis")]
    name: String,

    #[arg(long, default_value = "UTC")]
    timezone: String,

    #[arg(long, default_value_t = 2)]
    security_level: u8,
}

#[derive(Debug, Args)]
struct RunArgs {
    #[arg(long)]
    once: bool,

    #[arg(long)]
    force: bool,

    #[arg(long)]
    task: Option<String>,
}

pub fn run() -> Result<()> {
    let cli = Cli::parse();
    let output = execute(cli)?;
    println!("{output}");
    Ok(())
}

fn execute(cli: Cli) -> Result<String> {
    match cli.command {
        Commands::Init(args) => handle_init(cli.data_dir, args),
        Commands::Run(args) => handle_run(cli.data_dir, args),
        Commands::Status => handle_status(cli.data_dir),
        Commands::Doctor => handle_doctor(cli.data_dir),
    }
}

fn handle_init(data_dir_override: Option<PathBuf>, args: InitArgs) -> Result<String> {
    let data_dir = data_dir_override.unwrap_or(default_data_dir()?);
    let base_paths = PraxisPaths::for_data_dir(data_dir.clone());
    let clock = SystemClock::from_env()?;
    let now = clock.now_utc();

    let config = if base_paths.config_file.exists() {
        AppConfig::load(&base_paths.config_file)?.with_overridden_data_dir(data_dir.clone())
    } else {
        let mut config = AppConfig::default_for_data_dir(data_dir.clone());
        config.set_name(args.name);
        config.set_timezone(args.timezone)?;
        config.security.level = args.security_level;
        config.validate()?;
        config
    };

    let paths = PraxisPaths::from_config(&config);
    config.save(&paths.config_file)?;

    let store = SqliteSessionStore::new(paths.database_file.clone());
    store.initialize()?;

    let identity = LocalIdentityPolicy;
    identity.ensure_foundation(&paths, &config, now)?;

    Ok(format!(
        "initialized: ok\ndata_dir: {}\nconfig: {}\ndatabase: {}",
        paths.data_dir.display(),
        paths.config_file.display(),
        paths.database_file.display(),
    ))
}

fn handle_run(data_dir_override: Option<PathBuf>, args: RunArgs) -> Result<String> {
    let (config, paths) = load_initialized_config(data_dir_override)?;
    let identity = LocalIdentityPolicy;
    identity.validate(&paths)?;

    let store = SqliteSessionStore::new(paths.database_file.clone());
    store.initialize()?;
    store.validate_schema()?;

    let clock = SystemClock::from_env()?;
    let runtime = PraxisRuntime {
        config: &config,
        paths: &paths,
        backend: &StubBackend,
        clock: &clock,
        events: &NoopEventSink,
        goal_parser: &MarkdownGoalParser,
        identity: &identity,
        store: &store,
    };

    let summary = runtime.run_once(RunOptions {
        once: args.once,
        force: args.force,
        task: args.task,
    })?;

    let mut output = String::new();
    writeln!(output, "outcome: {}", summary.outcome)?;
    writeln!(output, "phase: {}", summary.phase)?;
    writeln!(output, "resumed: {}", summary.resumed)?;
    writeln!(
        output,
        "goal: {}",
        summary
            .selected_goal_id
            .as_deref()
            .zip(summary.selected_goal_title.as_deref())
            .map(|(id, title)| format!("{id}: {title}"))
            .unwrap_or_else(|| "-".to_string())
    )?;
    writeln!(
        output,
        "task: {}",
        summary.selected_task.as_deref().unwrap_or("-")
    )?;
    write!(output, "summary: {}", summary.action_summary)?;
    Ok(output)
}

fn handle_status(data_dir_override: Option<PathBuf>) -> Result<String> {
    let data_dir = data_dir_override.unwrap_or(default_data_dir()?);
    let base_paths = PraxisPaths::for_data_dir(data_dir.clone());
    if !base_paths.config_file.exists() {
        return Ok(format!(
            "status: uninitialized\ndata_dir: {}\nhint: run `praxis --data-dir {} init`",
            data_dir.display(),
            data_dir.display(),
        ));
    }

    let config = AppConfig::load(&base_paths.config_file)?.with_overridden_data_dir(data_dir);
    let paths = PraxisPaths::from_config(&config);
    let state = SessionState::load(&paths.state_file)?;
    let store = SqliteSessionStore::new(paths.database_file.clone());
    let last_session = store.last_session()?;

    let phase = state
        .as_ref()
        .map(|current| current.current_phase.to_string())
        .unwrap_or_else(|| SessionPhase::Sleep.to_string());
    let outcome = state
        .as_ref()
        .and_then(|current| current.last_outcome.clone())
        .unwrap_or_else(|| "none".to_string());

    let mut output = String::new();
    writeln!(output, "status: ready")?;
    writeln!(output, "data_dir: {}", paths.data_dir.display())?;
    writeln!(output, "phase: {phase}")?;
    writeln!(output, "last_outcome: {outcome}")?;

    if let Some(session) = last_session {
        writeln!(
            output,
            "last_session: #{} {}",
            session.session_num, session.outcome
        )?;
        writeln!(output, "last_session_ended_at: {}", session.ended_at)?;
    } else {
        writeln!(output, "last_session: none")?;
    }

    Ok(output)
}

fn handle_doctor(data_dir_override: Option<PathBuf>) -> Result<String> {
    let (config, paths) = load_initialized_config(data_dir_override)?;
    config.validate()?;

    let identity = LocalIdentityPolicy;
    identity.validate(&paths)?;

    let store = SqliteSessionStore::new(paths.database_file.clone());
    store.initialize()?;
    store.validate_schema()?;

    Ok(format!(
        "doctor: ok\nconfig: ok\nidentity: ok\ndatabase: ok\nbackend: {}",
        config.agent.backend
    ))
}

fn load_initialized_config(data_dir_override: Option<PathBuf>) -> Result<(AppConfig, PraxisPaths)> {
    let data_dir = data_dir_override.unwrap_or(default_data_dir()?);
    let base_paths = PraxisPaths::for_data_dir(data_dir.clone());

    if !base_paths.config_file.exists() {
        bail!(
            "Praxis is not initialized in {}. Run `praxis --data-dir {} init` first.",
            data_dir.display(),
            data_dir.display()
        );
    }

    let config = AppConfig::load(&base_paths.config_file)
        .with_context(|| format!("failed to load {}", base_paths.config_file.display()))?
        .with_overridden_data_dir(data_dir);
    let paths = PraxisPaths::from_config(&config);
    Ok((config, paths))
}
