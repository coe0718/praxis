use std::{fmt::Write as _, path::PathBuf};

use anyhow::{Context, Result, bail};

use crate::{
    backend::{AgentBackend, ConfiguredBackend},
    canary::ModelCanaryLedger,
    cli::{AskArgs, InitArgs, RunArgs},
    config::AppConfig,
    events::FileEventSink,
    heartbeat::write_heartbeat,
    identity::{IdentityPolicy, LocalIdentityPolicy, MarkdownGoalParser},
    r#loop::{PraxisRuntime, RunOptions},
    paths::{PraxisPaths, default_data_dir},
    profiles::ProfileSettings,
    providers::ProviderSettings,
    quality::{EvalRunner, LocalEvalSuite, LocalReviewer, Reviewer},
    report::{build_status_report, render_status_report},
    storage::{SessionStore, SqliteSessionStore},
    time::{Clock, SystemClock, parse_timezone},
    tools::{FileToolRegistry, ToolRegistry, sync_capabilities},
    usage::{UsageBudgetMode, UsageBudgetPolicy, estimate_tokens},
};

pub(crate) fn handle_init(data_dir_override: Option<PathBuf>, args: InitArgs) -> Result<String> {
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
    ProviderSettings::default().save_if_missing(&paths.providers_file)?;
    ProfileSettings::default().save_if_missing(&paths.profiles_file)?;
    UsageBudgetPolicy::default().save_if_missing(&paths.budgets_file)?;
    ModelCanaryLedger { records: Vec::new() }.save_if_missing(&paths.model_canary_file)?;

    let store = SqliteSessionStore::new(paths.database_file.clone());
    store.initialize()?;

    let identity = LocalIdentityPolicy;
    identity.ensure_foundation(&paths, &config, now)?;
    FileToolRegistry.ensure_foundation(&paths)?;
    sync_capabilities(&FileToolRegistry, &store, &paths)?;
    write_heartbeat(
        &paths.heartbeat_file,
        "praxis",
        "sleep",
        "Initialized data directory.",
        now,
    )?;

    Ok(format!(
        "initialized: ok\ndata_dir: {}\nconfig: {}\ndatabase: {}\ntools: {}",
        paths.data_dir.display(),
        paths.config_file.display(),
        paths.database_file.display(),
        paths.tools_dir.display(),
    ))
}

pub(crate) fn handle_run(data_dir_override: Option<PathBuf>, args: RunArgs) -> Result<String> {
    let (config, paths) = load_initialized_config(data_dir_override)?;
    let identity = LocalIdentityPolicy;
    let tools = FileToolRegistry;
    let backend = ConfiguredBackend::from_runtime(&config, &paths)?;
    let events = FileEventSink::new(paths.events_file.clone());

    identity.validate(&paths)?;
    tools.validate(&paths)?;

    let store = SqliteSessionStore::new(paths.database_file.clone());
    store.initialize()?;
    store.validate_schema()?;

    let clock = SystemClock::from_env()?;
    let runtime = PraxisRuntime {
        config: &config,
        paths: &paths,
        backend: &backend,
        clock: &clock,
        events: &events,
        goal_parser: &MarkdownGoalParser,
        identity: &identity,
        store: &store,
        tools: &tools,
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

pub(crate) fn handle_ask(data_dir_override: Option<PathBuf>, args: AskArgs) -> Result<String> {
    let prompt = args.prompt.join(" ").trim().to_string();
    if prompt.is_empty() {
        bail!("ask prompt must not be empty");
    }

    let (config, paths) = load_initialized_config(data_dir_override)?;
    let backend = ConfiguredBackend::from_runtime(&config, &paths)?;
    let budgets = UsageBudgetPolicy::load_or_default(&paths.budgets_file)?;
    let estimate = estimate_tokens(&prompt) + 220;
    let decision = budgets
        .rule(UsageBudgetMode::Ask)
        .check_estimate(estimate, UsageBudgetMode::Ask);

    let mut rendered = String::new();
    writeln!(rendered, "mode: ask")?;
    writeln!(rendered, "backend: {}", backend.name())?;
    writeln!(rendered, "stateful: false")?;
    writeln!(rendered, "prompt: {prompt}")?;
    if decision.blocked {
        write!(rendered, "answer: {}", decision.summary)?;
    } else {
        let output = backend.answer_prompt(&prompt)?;
        write!(rendered, "answer: {}", output.summary)?;
    }
    Ok(rendered)
}

pub(crate) fn handle_status(data_dir_override: Option<PathBuf>) -> Result<String> {
    let data_dir = data_dir_override.unwrap_or(default_data_dir()?);
    let base_paths = PraxisPaths::for_data_dir(data_dir.clone());
    if !base_paths.config_file.exists() {
        return Ok(format!(
            "status: uninitialized\ndata_dir: {}\nhint: run `praxis --data-dir {} init`",
            data_dir.display(),
            data_dir.display(),
        ));
    }

    let (config, paths) = load_initialized_config(Some(data_dir))?;
    Ok(render_status_report(&build_status_report(&config, &paths)?))
}

pub(crate) fn handle_doctor(data_dir_override: Option<PathBuf>) -> Result<String> {
    let (config, paths) = load_initialized_config(data_dir_override)?;
    config.validate()?;
    parse_timezone(&config.instance.timezone)?;
    ConfiguredBackend::validate_environment(&config, &paths)?;

    let identity = LocalIdentityPolicy;
    identity.validate(&paths)?;
    FileToolRegistry.validate(&paths)?;
    let providers = ProviderSettings::load_or_default(&paths.providers_file)?;
    providers.validate()?;
    ProfileSettings::load_or_default(&paths.profiles_file)?.validate()?;
    UsageBudgetPolicy::load_or_default(&paths.budgets_file)?.validate()?;
    ModelCanaryLedger::load_or_default(&paths.model_canary_file)?.validate()?;
    crate::heartbeat::read_heartbeat(&paths.heartbeat_file)?;
    let criteria_count = LocalReviewer.validate(&paths)?;
    let eval_count = LocalEvalSuite.validate(&paths)?;

    let store = SqliteSessionStore::new(paths.database_file.clone());
    store.initialize()?;
    store.validate_schema()?;

    Ok(format!(
        "doctor: ok\nconfig: ok\nidentity: ok\ndatabase: ok\ntools: ok\nproviders: ok\nbudgets: ok\nheartbeat: ok\nquality: ok\ngoal_criteria: {criteria_count}\nevals: {eval_count}\nbackend: {}",
        config.agent.backend
    ))
}

pub(crate) fn load_initialized_config(
    data_dir_override: Option<PathBuf>,
) -> Result<(AppConfig, PraxisPaths)> {
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
    let config = ProfileSettings::load_or_default(&paths.profiles_file)?.apply(&config)?;
    Ok((config, paths))
}
