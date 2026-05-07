use std::{fmt::Write as _, path::PathBuf};

use anyhow::{Context, Result, bail};

use crate::{
    archive::maybe_create_daily_snapshot,
    attachments::{AttachmentPolicy, render_attachments},
    backend::{AgentBackend, ConfiguredBackend},
    boundaries::BoundaryReviewState,
    canary::ModelCanaryLedger,
    cli::{AskArgs, InitArgs, RunArgs},
    config::{AppConfig, SecurityOverrides},
    events::FileEventSink,
    heartbeat::write_heartbeat,
    identity::{IdentityPolicy, LocalIdentityPolicy, MarkdownGoalParser},
    lite::LiteMode,
    r#loop::{PraxisRuntime, RunOptions},
    paths::{PraxisPaths, default_data_dir},
    plugins::PluginRegistry,
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

    // Detect init mode from args.
    let mode = crate::cli::wizard::WizardConfig::detect_mode(&args);

    let (config, api_key_hint) = if base_paths.config_file.exists() {
        // Existing config — reload and re-validate (idempotent).
        let config =
            AppConfig::load(&base_paths.config_file)?.with_overridden_data_dir(data_dir.clone());
        (config, None)
    } else if mode == crate::cli::wizard::InitMode::Wizard {
        // Run interactive wizard.
        let wiz = match crate::cli::wizard::run_wizard()? {
            Some(w) => w,
            None => return Ok("Setup cancelled.".to_string()),
        };
        let mut config = AppConfig::default_for_data_dir(data_dir.clone());
        config.set_name(wiz.name);
        config.set_timezone(wiz.timezone)?;
        config.agent.backend = wiz.backend.clone();
        config.security.level = wiz.security_level;
        config.validate()?;
        let hint = crate::cli::wizard::export_api_key_hint(&wiz.backend, wiz.api_key.as_deref());
        (config, Some(hint))
    } else {
        // Flag-driven init (existing behavior).
        let mut config = AppConfig::default_for_data_dir(data_dir.clone());
        config.set_name(args.name);
        config.set_timezone(args.timezone)?;
        config.security.level = args.security_level;
        config.validate()?;
        (config, None)
    };

    let paths = PraxisPaths::from_config(&config);
    config.save(&paths.config_file)?;
    ProviderSettings::default().save_if_missing(&paths.providers_file)?;
    ProfileSettings::default().save_if_missing(&paths.profiles_file)?;
    UsageBudgetPolicy::default().save_if_missing(&paths.budgets_file)?;
    ModelCanaryLedger { records: Vec::new() }.save_if_missing(&paths.model_canary_file)?;
    BoundaryReviewState::default().save_if_missing(&paths.boundary_review_file)?;

    let store = SqliteSessionStore::new(paths.database_file.clone());
    store.initialize()?;

    crate::crypto::load_or_generate_key(&paths.master_key_file)
        .context("failed to initialise at-rest encryption key")?;

    let identity = LocalIdentityPolicy;
    identity.ensure_foundation(&paths, &config, now)?;
    FileToolRegistry.ensure_foundation(&paths)?;
    sync_capabilities(&FileToolRegistry, &store, &paths)?;
    write_heartbeat(&paths.heartbeat_file, "praxis", "sleep", "Initialized data directory.", now)?;

    let mut lines = vec![
        format!("initialized: ok"),
        format!("data_dir: {}", paths.data_dir.display()),
        format!("config: {}", paths.config_file.display()),
        format!("database: {}", paths.database_file.display()),
        format!("tools: {}", paths.tools_dir.display()),
    ];
    if let Some(hint) = api_key_hint {
        lines.push(hint);
    }
    Ok(lines.join("\n"))
}

pub(crate) fn handle_run(data_dir_override: Option<PathBuf>, args: RunArgs) -> Result<String> {
    let (mut config, paths) = load_initialized_config(data_dir_override)?;
    if let Some(profile_name) = &args.profile {
        config.agent.profile = profile_name.clone();
        let profiles = ProfileSettings::load_or_default(&paths.profiles_file)?;
        config = profiles.apply(&config)?;
    }

    // #47 — CLI override: force secret redaction on for this run.
    if args.redact_secrets {
        config.security.redact_secrets = true;
    }

    let identity = LocalIdentityPolicy;
    let tools = FileToolRegistry;
    let backend = ConfiguredBackend::from_runtime(&config, &paths)?;
    let events = FileEventSink::new(paths.events_file.clone());

    identity.validate(&paths)?;
    tools.validate(&paths)?;

    let vault = crate::vault::Vault::load(&paths.vault_file).unwrap_or_default();
    for warning in crate::vault::audit_literals(&vault) {
        log::warn!("{warning}");
    }

    let store = SqliteSessionStore::new(paths.database_file.clone());
    store.initialize()?;
    store.validate_schema()?;

    let clock = SystemClock::from_env()?;
    let mut lite = LiteMode::from_file(&paths.config_file).unwrap_or_default();
    // #23 — CLI flag: activate fast mode for this run.
    if args.fast || LiteMode::is_fast_active(&paths.data_dir) {
        lite = LiteMode::fast_all();
    }
    let process_manager = crate::process_manager::ProcessManager::new();
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
        lite: &lite,
        last_tool_activity: std::cell::Cell::new(None),
        plugins: std::cell::RefCell::new(PluginRegistry::new(&paths)),
        process_manager: &process_manager,
    };

    // #7 — one-shot: force a single pass with no loop continuation.
    let (once, force) = if args.one_shot {
        (true, true)
    } else {
        (args.once, args.force)
    };

    let summary = tokio::runtime::Runtime::new()
        .context("failed to create tokio runtime")?
        .block_on(runtime.run_once(RunOptions { once, force, task: args.task }))?;
    let snapshot = if config.runtime.daily_backup_snapshots {
        maybe_create_daily_snapshot(&config, &paths, clock.now_utc())?
    } else {
        None
    };

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
    writeln!(output, "task: {}", summary.selected_task.as_deref().unwrap_or("-"))?;
    if let Some(snapshot) = snapshot {
        writeln!(
            output,
            "snapshot: created {} pruned={}",
            snapshot.output_dir.display(),
            snapshot.pruned
        )?;
    }
    write!(output, "summary: {}", summary.action_summary)?;
    Ok(output)
}

pub(crate) fn handle_ask(data_dir_override: Option<PathBuf>, args: AskArgs) -> Result<String> {
    let base_prompt = args.prompt.join(" ").trim().to_string();
    if base_prompt.is_empty() {
        bail!("ask prompt must not be empty");
    }
    let policy = AttachmentPolicy::parse(&args.attachment_policy)?;
    let attachments = render_attachments(&args.files, policy)?;
    let prompt = if attachments.is_empty() {
        base_prompt
    } else {
        format!("{base_prompt}\n\nAttached files:\n{attachments}")
    };

    // One-shot mode — run the full agent loop.
    // Enabled by: tools=true (default), --one-shot / -z, or --no-tools=false.
    // Disabled by: --no-tools.
    let use_tools = !args.no_tools;

    let (mut config, paths) = load_initialized_config(data_dir_override)?;

    // #47 — CLI override: force secret redaction on for this invocation.
    if args.redact_secrets {
        config.security.redact_secrets = true;
    }

    let backend = ConfiguredBackend::from_runtime(&config, &paths)?;
    let budgets = UsageBudgetPolicy::load_or_default(&paths.budgets_file)?;
    let estimate = estimate_tokens(&prompt) + 220;
    let decision = budgets
        .rule(UsageBudgetMode::Ask)
        .check_estimate(estimate, UsageBudgetMode::Ask);

    if decision.blocked && !use_tools {
        let mut rendered = String::new();
        writeln!(rendered, "mode: ask")?;
        writeln!(rendered, "backend: {}", backend.name())?;
        writeln!(rendered, "attachments: {}", args.files.len())?;
        writeln!(rendered, "prompt: {prompt}")?;
        write!(rendered, "answer: {}", decision.summary)?;
        return Ok(rendered);
    }

    // One-shot mode with tools — run the full agent loop.
    if use_tools {
        let identity = LocalIdentityPolicy;
        let tools = FileToolRegistry;
        let events = FileEventSink::new(paths.events_file.clone());

        identity.validate(&paths)?;
        tools.validate(&paths)?;

        let store = SqliteSessionStore::new(paths.database_file.clone());
        store.initialize()?;
        store.validate_schema()?;

        let clock = SystemClock::from_env()?;
        let lite = LiteMode::from_file(&paths.config_file).unwrap_or_default();
        let process_manager = crate::process_manager::ProcessManager::new();
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
            lite: &lite,
            last_tool_activity: std::cell::Cell::new(None),
            plugins: std::cell::RefCell::new(PluginRegistry::new(&paths)),
            process_manager: &process_manager,
        };

        let summary = tokio::runtime::Runtime::new()
            .context("failed to create tokio runtime")?
            .block_on(runtime.run_once(RunOptions {
                once: true,
                force: true,
                task: Some(prompt.clone()),
            }))?;

        let mut rendered = String::new();
        writeln!(rendered, "mode: ask --tools")?;
        writeln!(rendered, "backend: {}", backend.name())?;
        writeln!(rendered, "stateful: true")?;
        writeln!(rendered, "attachments: {}", args.files.len())?;
        writeln!(rendered, "prompt: {prompt}")?;
        writeln!(rendered, "outcome: {}", summary.outcome)?;
        writeln!(rendered, "phase: {}", summary.phase)?;
        write!(rendered, "summary: {}", summary.action_summary)?;
        return Ok(rendered);
    }

    // Simple one-shot — single LLM call, no tools.
    let mut rendered = String::new();
    writeln!(rendered, "mode: ask")?;
    writeln!(rendered, "backend: {}", backend.name())?;
    writeln!(rendered, "stateful: false")?;
    writeln!(rendered, "attachments: {}", args.files.len())?;
    writeln!(rendered, "prompt: {prompt}")?;
    let output = backend.answer_prompt(&prompt)?;
    write!(rendered, "answer: {}", output.summary)?;
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
    BoundaryReviewState::load_or_default(&paths.boundary_review_file)?.validate()?;
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

    let mut config = AppConfig::load(&base_paths.config_file)
        .with_context(|| format!("failed to load {}", base_paths.config_file.display()))?
        .with_overridden_data_dir(data_dir);
    let paths = PraxisPaths::from_config(&config);

    // Apply sensitive overrides from `security.toml` (gitignored, operator-only).
    let security = SecurityOverrides::load_or_default(&paths.security_file)?;
    if let Some(level) = security.level {
        if !(1..=4).contains(&level) {
            bail!("security override level must be between 1 and 4, got {level}");
        }
        config.security.level = level;
    }

    let config = ProfileSettings::load_or_default(&paths.profiles_file)?.apply(&config)?;
    Ok((config, paths))
}
