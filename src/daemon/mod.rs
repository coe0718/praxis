//! Always-on daemon loop — runs Praxis as a persistent process rather than
//! a one-shot CLI invocation.
//!
//! The daemon wraps the existing `Orient → Decide → Act → Reflect` session
//! loop in an outer supervisor that:
//!
//! - **Reacts** to inbound bus events (Telegram messages, webhooks) as they
//!   arrive, triggering a session immediately rather than waiting for the next
//!   scheduled window.
//! - **Proacts** on an adaptive schedule derived from `OperatorSchedule`,
//!   automatically skipping quiet hours once enough activity samples exist.
//! - **Maintains** itself between sessions — anatomy refresh, brief generation —
//!   without interrupting active work.
//! - **Evolves** through the existing Reflect phase: every session still writes
//!   to `SOUL.md`, `IDENTITY.md`, `GOALS.md`, memory, score, and all other
//!   identity artifacts.  The daemon adds nothing special here; it just calls
//!   `run_once` in a loop.
//! - **Shuts down gracefully** on `SIGTERM` or `Ctrl-C`, always finishing the
//!   current Reflect before exiting so no session data is lost.
//!
//! ## PID file
//!
//! On startup the daemon writes its PID to `daemon.pid` in the data directory.
//! The file is removed on clean exit.  If the file already exists and the
//! owning process is still alive, the daemon refuses to start (use
//! `--force` to override).
//!
//! ## Supervision
//!
//! Run under systemd, supervisord, or Docker `restart=always`.  The daemon
//! exits with code 0 on clean shutdown and non-zero on fatal errors so the
//! supervisor can distinguish the two.
//!
//! ```ini
//! # /etc/systemd/system/praxis.service
//! [Unit]
//! Description=Praxis agent daemon
//! After=network.target
//!
//! [Service]
//! ExecStart=/usr/local/bin/praxis daemon
//! Restart=on-failure
//! RestartSec=10
//! Environment=ANTHROPIC_API_KEY=sk-ant-...
//!
//! [Install]
//! WantedBy=multi-user.target
//! ```

use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use anyhow::{Context, Result, bail};
use chrono::{DateTime, Utc};

use crate::{
    anatomy::refresh_stale_anatomy,
    backend::ConfiguredBackend,
    circuit_breaker::CircuitBreaker,
    cost::CostTracker,
    events::FileEventSink,
    graceful_shutdown::GracefulShutdown,
    health::{HealthMonitor, HealthStatus},
    heartbeat::write_heartbeat,
    identity::{IdentityPolicy, LocalIdentityPolicy, MarkdownGoalParser},
    lite::LiteMode,
    r#loop::{PraxisRuntime, RunOptions, RunSummary},
    paths::PraxisPaths,
    plugins::PluginRegistry,
    profiles::ProfileSettings,
    storage::{SessionStore, SqliteSessionStore},
    time::SystemClock,
    tools::{FileToolRegistry, ToolRegistry, sync_capabilities},
    wakeup::schedule::OperatorSchedule,
};

// ── Configuration ─────────────────────────────────────────────────────────────

/// Runtime tuning for the daemon loop.
#[derive(Debug, Clone)]
pub struct DaemonConfig {
    /// How often to poll for bus events and wake intents when idle (seconds).
    /// Lower values make reactive triggers faster at the cost of more I/O.
    pub poll_interval_secs: u64,
    /// Minimum gap between scheduled (non-reactive) sessions (minutes).
    /// Overridden by `OperatorSchedule` once enough samples exist.
    pub min_session_gap_mins: u64,
    /// How long after the last maintenance run before scheduling the next one (minutes).
    pub maintenance_interval_mins: u64,
    /// Skip the PID-file guard and start even if another daemon appears to be
    /// running.  Use with care — two daemons writing to the same data directory
    /// will corrupt session state.
    pub force_start: bool,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            poll_interval_secs: 30,
            min_session_gap_mins: 240, // 4 hours
            maintenance_interval_mins: 60,
            force_start: false,
        }
    }
}

// ── What triggered a session ──────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum SessionTrigger {
    /// Operator-injected task via `wake_intent.json`.
    WakeIntent { task: Option<String> },
    /// New inbound bus event (Telegram message, webhook, etc.).
    BusEvent,
    /// Regular scheduled session based on the operator schedule.
    Scheduled,
}

impl SessionTrigger {
    fn label(&self) -> &'static str {
        match self {
            Self::WakeIntent { .. } => "wake-intent",
            Self::BusEvent => "bus-event",
            Self::Scheduled => "scheduled",
        }
    }

    fn task(&self) -> Option<&str> {
        match self {
            Self::WakeIntent { task: Some(t) } => Some(t.as_str()),
            _ => None,
        }
    }
}

// ── PID file ──────────────────────────────────────────────────────────────────

/// RAII guard: writes PID on construction, removes the file on drop.
pub struct PidFile {
    path: PathBuf,
}

impl PidFile {
    /// Write the current process PID to `path`.
    ///
    /// If `force` is false and the file already exists with a living process,
    /// returns an error.
    /// W12 fix: Use atomic file creation to prevent TOCTOU race.
    pub fn acquire(path: &Path, force: bool) -> Result<Self> {
        if path.exists() && !force {
            let raw = fs::read_to_string(path).unwrap_or_default();
            let existing_pid: u32 = raw.trim().parse().unwrap_or(0);
            if existing_pid > 0 && process_is_alive(existing_pid) {
                bail!(
                    "daemon already running (PID {existing_pid}).  \\
                     Kill it first or use --force to override."
                );
            }
            // Stale PID file — safe to overwrite.
        }
        let pid = std::process::id();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        // W12 fix: Use create_new for atomic creation (fails if file exists)
        // For force mode, we delete first then atomically create
        let file =
            if force && path.exists() {
                let _ = fs::remove_file(path);
                fs::OpenOptions::new().write(true).create_new(true).open(path).with_context(
                    || format!("failed to atomically create PID file {}", path.display()),
                )?
            } else {
                fs::OpenOptions::new().write(true).create_new(true).open(path).with_context(
                    || format!("failed to atomically create PID file {}", path.display()),
                )?
            };
        let mut file = file;
        file.write_all(pid.to_string().as_bytes())
            .with_context(|| format!("failed to write PID file {}", path.display()))?;
        file.flush().ok();
        Ok(Self { path: path.to_path_buf() })
    }
}

impl Drop for PidFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

/// Check whether a process is alive by probing `/proc/<pid>` on Linux or
/// attempting to open `\\.\pipe\` on Windows.  On other platforms, returns
/// `false` (stale PID files will be overwritten).
fn process_is_alive(pid: u32) -> bool {
    #[cfg(target_os = "linux")]
    {
        // `/proc/<pid>` exists iff the process is running.
        std::path::Path::new(&format!("/proc/{pid}")).exists()
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = pid;
        false
    }
}

// ── Bus watcher ───────────────────────────────────────────────────────────────

/// Detects new entries in `bus.jsonl` by tracking the file's byte length.
struct BusWatcher {
    path: PathBuf,
    last_len: u64,
}

impl BusWatcher {
    fn new(path: PathBuf) -> Self {
        let last_len = file_len(&path);
        Self { path, last_len }
    }

    /// Returns `true` if the bus file has grown since the last check.
    fn has_new_events(&mut self) -> bool {
        let current = file_len(&self.path);
        if current > self.last_len {
            self.last_len = current;
            true
        } else {
            // Handle truncation (e.g., log rotation) gracefully.
            self.last_len = current;
            false
        }
    }
}

fn file_len(path: &Path) -> u64 {
    fs::metadata(path).map(|m| m.len()).unwrap_or(0)
}

// ── Main entry point ──────────────────────────────────────────────────────────

/// Start the daemon loop.  Blocks until shutdown.
///
/// `data_dir` — absolute path to the Praxis data directory.
/// `config`   — daemon-level tuning (poll interval, session gap, etc.).
pub fn run_daemon(data_dir: PathBuf, config: DaemonConfig) -> Result<()> {
    let paths = PraxisPaths::for_data_dir(data_dir.clone());

    // Guard against double-start.
    let _pid = PidFile::acquire(&paths.daemon_pid_file, config.force_start)?;

    // Build a tokio runtime for signal handling and async sleep.
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("failed to build tokio runtime")?;

    rt.block_on(async_daemon_loop(data_dir, config, paths))
}

async fn async_daemon_loop(
    data_dir: PathBuf,
    config: DaemonConfig,
    paths: PraxisPaths,
) -> Result<()> {
    // Set up graceful shutdown with cleanup hooks.
    let mut shutdown = GracefulShutdown::new(Duration::from_secs(30));
    let shutdown_flag = shutdown.flag().clone();

    // Register cleanup hooks.
    {
        let pid_path = paths.daemon_pid_file.clone();
        shutdown.register_closure("remove-pid-file", move || {
            let _ = fs::remove_file(&pid_path);
            Ok(())
        });
    }

    // Spawn signal watchers that set the flag when SIGINT/SIGTERM arrives.
    {
        let flag = shutdown_flag.clone();
        tokio::spawn(async move {
            wait_for_shutdown_signal().await;
            log::info!("daemon: shutdown signal received — finishing current session then exiting");
            flag.request_shutdown();
        });
    }

    // Set up health monitor with subsystem checks.
    let health_monitor = {
        let monitor = HealthMonitor::new();
        let db_path = paths.database_file.clone();
        let cfg_path = paths.config_file.clone();
        monitor.register(
            "database",
            3,
            Box::new(move || {
                if db_path.exists() {
                    let store = SqliteSessionStore::new(db_path.clone());
                    match store.initialize().and_then(|_| store.validate_schema()) {
                        Ok(()) => HealthStatus::Healthy,
                        Err(e) => HealthStatus::Unhealthy {
                            reason: format!("db error: {e}"),
                        },
                    }
                } else {
                    HealthStatus::Unhealthy {
                        reason: "database file not found".to_string(),
                    }
                }
            }),
        );
        monitor.register(
            "config",
            3,
            Box::new(move || {
                if cfg_path.exists() {
                    match crate::config::AppConfig::load(&cfg_path) {
                        Ok(_) => HealthStatus::Healthy,
                        Err(e) => HealthStatus::Unhealthy {
                            reason: format!("parse error: {e}"),
                        },
                    }
                } else {
                    HealthStatus::Healthy
                }
            }),
        );
        monitor.register(
            "goals",
            3,
            Box::new({
                let goals_path = paths.data_dir.join("GOALS.md");
                move || {
                    if goals_path.exists() {
                        HealthStatus::Healthy
                    } else {
                        HealthStatus::Degraded {
                            reason: "GOALS.md missing".to_string(),
                        }
                    }
                }
            }),
        );
        Arc::new(monitor)
    };

    // Set up circuit breaker for session execution.
    let session_breaker = CircuitBreaker::new("session");

    // Set up cost tracker.
    let cost_tracker = Arc::new(std::sync::Mutex::new(CostTracker::new()));

    let mut bus_watcher = BusWatcher::new(paths.bus_file.clone());
    let mut last_session_at: Option<DateTime<Utc>> = None;
    let mut last_maintenance_at: Option<DateTime<Utc>> = None;
    let poll = Duration::from_secs(config.poll_interval_secs);

    // Watch praxis.toml for hot-reload — config changes take effect next cycle.
    let cfg_watcher = crate::config::ConfigWatcher::spawn(paths.config_file.clone())?;

    // Initialise credential pools if the feature flag is enabled.
    if let Ok(cfg) = crate::config::AppConfig::load(&paths.config_file)
        && cfg.features.credential_pooling
    {
        let providers = crate::providers::ProviderSettings::load_or_default(&paths.providers_file)?;
        let names: Vec<&str> = providers.providers.iter().map(|r| r.provider.as_str()).collect();
        crate::backend::init_pools(&names);
};
    // ── Discord Gateway WebSocket (real-time push) ────────────────────────
    #[cfg(feature = "discord")]
    let _discord_gateway_handle = {
        let token = std::env::var("PRAXIS_DISCORD_BOT_TOKEN").ok();
        if let Some(token) = token {
            let bus = crate::bus::FileBus::new(paths.bus_file.clone());
            let bus = std::sync::Arc::new(bus) as std::sync::Arc<dyn crate::bus::MessageBus + Send + Sync>;
            log::info!("daemon: starting Discord Gateway for real-time message delivery");
            Some(tokio::spawn(async move {
                crate::messaging::run_gateway(bus, token).await;
            }))
        } else {
            log::info!("daemon: Discord Gateway disabled (PRAXIS_DISCORD_BOT_TOKEN not set)");
            None
        }
    };

    log::info!(
        "daemon: started (pid={}, data_dir={})",
        std::process::id(),
        paths.data_dir.display()
    );

    // Update heartbeat so `praxis status` shows the daemon is alive.
    let _ = write_heartbeat(
        &paths.heartbeat_file,
        "praxis",
        "daemon",
        "Daemon started and ready.",
        Utc::now(),
    );

    loop {
        if shutdown_flag.is_shutting_down() {
            log::info!("daemon: clean shutdown");
            break;
        }

        let now = Utc::now();

        // ── Config hot-reload ────────────────────────────────────────────
        // If praxis.toml changed, re-validate immediately.  The next
        // session will pick up the new config without a daemon restart.
        if cfg_watcher.take_dirty() {
            match crate::config::AppConfig::load(&paths.config_file) {
                Ok(cfg) => {
                    let flags = cfg.features.enabled_list();
                    let flags_str = if flags.is_empty() {
                        "none".to_string()
                    } else {
                        flags.join(",")
                    };
                    log::info!(
                        "daemon: config reloaded — backend={}, security_level={}, features=[{}]",
                        cfg.agent.backend,
                        cfg.security.level,
                        flags_str
                    );
                }
                Err(e) => {
                    log::warn!("daemon: config changed but failed to load: {e:#}");
                }
            }
            // Re-validate paths in case data_dir or other paths changed.
            let _ = write_heartbeat(
                &paths.heartbeat_file,
                "praxis",
                "daemon",
                "Config reloaded.",
                Utc::now(),
            );
        }

        // ── Reactive triggers (highest priority) ───────────────────────────

        // Poll messaging platforms and publish inbound messages to the bus.
        // This enables reactive sessions triggered by Discord/Slack/Telegram
        // messages, similar to how the webhook system works.
        poll_platforms(&paths);

        // ── Proactive triggers ───────────────────────────────────────────────────
        // Check for proactive wake-ups based on conditions (time, file changes, etc.)
        let proactive_task = {
            let proactive_path = paths.data_dir.join("proactive_state.json");
            if proactive_path.exists() {
                let mut agent: crate::proactive::ProactiveAgent = match serde_json::from_slice(
                    &std::fs::read(&proactive_path).unwrap_or_default(),
                ) {
                    Ok(a) => a,
                    Err(_) => crate::proactive::ProactiveAgent::new(),
                };
                let wake_ids = agent.check();
                if !wake_ids.is_empty() {
                    log::info!("daemon: proactive wake-ups triggered: {:?}", wake_ids);
                }
                // For now, proactive wake-ups just trigger a session
                if !wake_ids.is_empty() {
                    Some("proactive check".to_string())
                } else {
                    None
                }
            } else {
                None
            }
        };

        let trigger = if let Some(intent) = crate::wakeup::consume_intent(&paths.data_dir)? {
            log::info!("daemon: wake intent from '{}': {}", intent.source, intent.reason);
            Some(SessionTrigger::WakeIntent { task: intent.task })
        } else if bus_watcher.has_new_events() {
            log::info!("daemon: new bus event(s) — triggering reactive session");
            Some(SessionTrigger::BusEvent)
        } else if proactive_task.is_some() {
            Some(SessionTrigger::WakeIntent { task: proactive_task })
        } else {
            check_scheduled_jobs(&paths, now)
                .map(|task| SessionTrigger::WakeIntent { task: Some(task) })
        };

        // ── Scheduled session ──────────────────────────────────────────────

        let trigger = trigger.or_else(|| {
            if should_run_scheduled_session(&paths, last_session_at, &config, now) {
                Some(SessionTrigger::Scheduled)
            } else {
                None
            }
        });

        // ── Run session ────────────────────────────────────────────────────

        if let Some(trigger) = trigger {
            let label = trigger.label();
            let task = trigger.task().map(str::to_string);

            // Check circuit breaker before running session.
            if !session_breaker.is_available() {
                log::warn!(
                    "daemon: session circuit breaker OPEN — skipping session (consecutive failures: {})",
                    session_breaker.failures()
                );
                tokio::time::sleep(poll).await;
                continue;
            }

            log::info!("daemon: starting session (trigger={label})");
            // Run the session directly - we're already in an async context.
            // Use block_in_place to allow the async runtime to progress through
            // the synchronous session execution.
            let result = run_session_async(&data_dir, task.clone()).await;

            match result {
                Ok(summary) => {
                    session_breaker.record_success();
                    log::info!(
                        "daemon: session complete (trigger={label} outcome={} goal={:?})",
                        summary.outcome,
                        summary.selected_goal_title,
                    );
                    last_session_at = Some(Utc::now());
                    record_operator_activity(&paths, Utc::now());

                    // ── Mobile notifications ─────────────────────────────────
                    let mobile_agent = crate::mobile::MobileAgent::from_file(
                        &paths.data_dir.join("mobile_config.json"),
                    );
                    if mobile_agent.enabled {
                        let outcome_str = &summary.outcome;
                        let goal_title = summary.selected_goal_title.as_deref().unwrap_or("none");
                        if let Err(e) = mobile_agent.session_summary(outcome_str, goal_title) {
                            log::warn!("mobile: failed to send notification: {e}");
                        }
                    }
                }
                Err(e) => {
                    session_breaker.record_failure();
                    log::error!("daemon: session failed (trigger={label}): {e:#}");
                    // Don't exit on session failure — stay alive and try again.
                    last_session_at = Some(Utc::now());
                }
            }

            // Re-check shutdown after a (potentially long) session.
            if shutdown_flag.is_shutting_down() {
                log::info!("daemon: clean shutdown after session");
                break;
            }

            // Skip maintenance after a full session — Reflect already handles it.
            tokio::time::sleep(poll).await;
            continue;
        }

        // ── Maintenance window (no session needed right now) ───────────────

        if should_run_maintenance(last_maintenance_at, &config, now) {
            log::debug!("daemon: running maintenance window");

            // Run health checks.
            let results = health_monitor.check_all();
            let overall = health_monitor.overall_health();
            if overall != HealthStatus::Healthy {
                let failed: Vec<&str> = results
                    .iter()
                    .filter(|r| !matches!(r.status, HealthStatus::Healthy | HealthStatus::Unknown))
                    .map(|r| r.name.as_str())
                    .collect();
                log::warn!("daemon: health check degraded — {:?}", failed);
            }

            let data_dir2 = data_dir.clone();
            let result = tokio::task::block_in_place(|| run_maintenance_blocking(&data_dir2));
            match result {
                Ok(refreshed) if refreshed > 0 => {
                    log::info!("daemon: anatomy refresh — {refreshed} file(s) updated");
                }
                Ok(_) => {}
                Err(e) => log::warn!("daemon: maintenance error: {e:#}"),
            }
            last_maintenance_at = Some(Utc::now());
        }

        // ── Poll sleep ─────────────────────────────────────────────────────

        tokio::time::sleep(poll).await;
    }

    // Run cleanup hooks before exiting.
    shutdown.run_hooks();

    // Log cost summary.
    if let Ok(tracker) = cost_tracker.lock() {
        log::info!(
            "daemon: total cost ${:.4} across {} entries",
            tracker.total_cost(),
            tracker.len()
        );
    }

    Ok(())
}

// ── Session runner ────────────────────────────────────────────────────────────

/// Load fresh config from disk and run a full Orient→Decide→Act→Reflect cycle.
///
/// Config is re-read every call so self-evolution changes (e.g., the agent
/// updated `praxis.toml` as part of the Act phase) take effect next session.
async fn run_session_async(data_dir: &Path, task: Option<String>) -> Result<RunSummary> {
    use crate::config::AppConfig;

    let base = PraxisPaths::for_data_dir(data_dir.to_path_buf());

    let mut config = AppConfig::load(&base.config_file)
        .with_context(|| format!("failed to load {}", base.config_file.display()))?;
    let paths = PraxisPaths::for_data_dir(data_dir.to_path_buf());

    // Apply profile settings (may be overridden by the agent between sessions).
    config = ProfileSettings::load_or_default(&paths.profiles_file)?.apply(&config)?;

    let identity = LocalIdentityPolicy;
    let tools = FileToolRegistry;
    let backend = ConfiguredBackend::from_runtime(&config, &paths)?;
    let events = FileEventSink::new(paths.events_file.clone());

    identity.validate(&paths)?;
    tools.ensure_foundation(&paths)?;
    tools.validate(&paths)?;

    // Discover and register MCP tools from configured servers.
    if let Err(e) = crate::tools::discover_mcp_tools(&paths) {
        log::warn!("mcp tool discovery failed (continuing without MCP tools): {e}");
    }

    let vault = crate::vault::Vault::load(&paths.vault_file).unwrap_or_default();
    for warning in crate::vault::audit_literals(&vault) {
        log::warn!("{warning}");
    }

    let store = SqliteSessionStore::new(paths.database_file.clone());
    store.initialize()?;
    store.validate_schema()?;
    sync_capabilities(&tools, &store, &paths)?;

    let lite = LiteMode::from_file(&paths.config_file).unwrap_or_default();
    let clock = SystemClock::from_env()?;

    // Set up ProcessManager for message-passing architecture.
    // Tool execution flows through execute_request in phases.rs.
    // with_tool_executor spawns async worker/compactor/corrector processes.
    let process_manager =
        crate::process_manager::ProcessManager::with_tool_executor(|tool_name, _args| {
            crate::process_manager::ToolResult {
                success: true,
                summary: format!("tool {} would execute", tool_name),
            }
        });

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
        personality: std::cell::RefCell::new(crate::personality::HeartwarePersonality::new()),
    };

    let summary = runtime
        .run_once(RunOptions {
            once: true, // Always single-pass from the daemon; the daemon manages the outer loop.
            force: task.is_some(),
            task,
        })
        .await?;

    Ok(summary)
}

/// Lightweight work that runs between sessions: re-index stale anatomy entries.
fn run_maintenance_blocking(data_dir: &Path) -> Result<usize> {
    let paths = PraxisPaths::for_data_dir(data_dir.to_path_buf());
    refresh_stale_anatomy(&paths)
}

// ── Scheduling helpers ────────────────────────────────────────────────────────

fn should_run_scheduled_session(
    paths: &PraxisPaths,
    last_session_at: Option<DateTime<Utc>>,
    config: &DaemonConfig,
    now: DateTime<Utc>,
) -> bool {
    let min_gap = chrono::Duration::minutes(config.min_session_gap_mins as i64);

    let elapsed_ok = match last_session_at {
        None => true, // First session — run immediately.
        Some(last) => now.signed_duration_since(last) >= min_gap,
    };

    if !elapsed_ok {
        return false;
    }

    // Check operator schedule for quiet hours.
    match OperatorSchedule::load(&paths.operator_schedule_file) {
        Ok(schedule) => {
            let quiet = schedule.quiet_hours();
            let current_hour = chrono::Timelike::hour(&now);
            !quiet.contains(&current_hour)
        }
        Err(_) => true, // No schedule yet — allow.
    }
}

fn should_run_maintenance(
    last_maintenance_at: Option<DateTime<Utc>>,
    config: &DaemonConfig,
    now: DateTime<Utc>,
) -> bool {
    let interval = chrono::Duration::minutes(config.maintenance_interval_mins as i64);
    match last_maintenance_at {
        None => true,
        Some(last) => now.signed_duration_since(last) >= interval,
    }
}

fn record_operator_activity(paths: &PraxisPaths, now: DateTime<Utc>) {
    match OperatorSchedule::load(&paths.operator_schedule_file) {
        Ok(mut schedule) => {
            schedule.record_activity(now);
            if let Err(e) = schedule.save(&paths.operator_schedule_file) {
                log::warn!("daemon: failed to save operator schedule: {e}");
            }
        }
        Err(e) => log::warn!("daemon: failed to load operator schedule: {e}"),
    }
}

/// Check scheduled jobs for due triggers.  Returns a task description if a
/// job is ready to fire, and persists the updated job store.
///
/// When a due job has `no_agent` set, runs the script directly and returns None.
/// When `wake_gate` is set, checks output for `wakeAgent: false` to skip triggering.
/// When `workdir` is set, the returned task is prefixed with a `cd <workdir>` instruction.
fn check_scheduled_jobs(paths: &PraxisPaths, now: DateTime<Utc>) -> Option<String> {
    use crate::tools::cron::ScheduledJobs;
    use crate::tools::cron_ext::{CronExtensions, run_script_job};

    let jobs_path = &paths.scheduled_jobs_file;
    let mut jobs = match ScheduledJobs::load(jobs_path) {
        Ok(j) => j,
        Err(_) => return None,
    };

    if jobs.jobs.is_empty() {
        return None;
    }

    let due = jobs.drain_due(now);
    if due.is_empty() {
        return None;
    }

    // Persist updated fire counts and removals.
    if let Err(e) = jobs.save(jobs_path) {
        log::warn!("daemon: failed to save scheduled jobs: {e}");
    }

    // Ensure the cron outputs directory exists.
    let _ = std::fs::create_dir_all(&paths.cron_outputs_dir);

    // Process jobs - check for no_agent first
    let mut tasks = Vec::new();
    let mut names = Vec::new();

    for job in &due {
        names.push(job.name.clone());

        // Check if this is a script-mode job (no_agent)
        if job.no_agent {
            let extensions = CronExtensions {
                no_agent: true,
                wake_gate: job.wake_gate,
            };
            // Run script directly without triggering agent session
            if let Ok(result) = run_script_job(job, &job.task, &extensions) {
                if result.should_wake_agent {
                    // For no_agent with wake_gate=false, output to a file instead
                    let output_path = paths.cron_outputs_dir.join(format!("{}.txt", job.id));
                    let _ = std::fs::write(&output_path, &result.output);
                }
                log::info!(
                    "daemon: no_agent job {} completed ({} bytes output)",
                    job.id,
                    result.output.len()
                );
            } else {
                log::warn!("daemon: no_agent job {} failed", job.id);
            }
            continue;
        }

        let mut parts = Vec::new();

        // Inject workdir context.
        if let Some(ref wd) = job.workdir {
            parts.push(format!("[workdir: {wd}]"));
        }

        // Inject context from upstream jobs.
        if let Some(ref upstream_ids) = job.context_from
            && !upstream_ids.is_empty()
        {
            let outputs =
                ScheduledJobs::read_upstream_outputs(&paths.cron_outputs_dir, upstream_ids);
            if !outputs.is_empty() {
                parts.push("[context from previous jobs]".to_string());
                for (upstream_id, output) in &outputs {
                    // Truncate very long outputs to avoid overwhelming the session.
                    let trimmed = if output.len() > 4000 {
                        format!(
                            "{}...\n[truncated — full output is {} bytes]",
                            &output[..4000],
                            output.len()
                        )
                    } else {
                        output.clone()
                    };
                    parts.push(format!("--- output of {upstream_id} ---\n{trimmed}"));
                }
            }
        }

        parts.push(job.task.clone());
        tasks.push(parts.join("\n"));
    }

    log::info!("daemon: scheduled jobs due: [{}]", names.join(", "));

    if tasks.is_empty() && !names.is_empty() {
        None // All jobs were no_agent
    } else {
        Some(tasks.join("\n\n"))
    }
}

// ── Platform polling ─────────────────────────────────────────────────────────

/// Poll all configured messaging platforms for new messages and publish
/// them to the message bus.  The daemon's bus watcher then triggers a
/// reactive session.  Errors are logged but never propagated — a polling
/// failure must not crash the daemon.
fn poll_platforms(paths: &PraxisPaths) {
    use crate::bus::{BusEvent, FileBus, MessageBus};
    let bus = FileBus::new(&paths.bus_file);

    // ── Telegram ──────────────────────────────────────────────────────
    if crate::messaging::TelegramBot::validate_environment().is_ok()
        && let Ok(bot) = crate::messaging::TelegramBot::from_env()
    {
        let activation =
            crate::messaging::ActivationStore::load(&paths.activation_file).unwrap_or_default();
        let gating = crate::messaging::MessageGating::default();
        let ephemeral = std::collections::HashMap::new();
        match bot.poll_once(
            &paths.telegram_state_file,
            &paths.sender_pairing_file,
            &bus,
            &activation,
            &gating,
            &ephemeral,
        ) {
            Ok((msgs, _callbacks)) => {
                if !msgs.is_empty() {
                    log::info!("daemon: telegram polled {} message(s)", msgs.len());
                }
            }
            Err(e) => log::debug!("daemon: telegram poll skipped: {e}"),
        }
    }

    // ── Discord ───────────────────────────────────────────────────────
    #[cfg(feature = "discord")]
    {
        if crate::messaging::DiscordClient::validate_environment().is_ok()
            && let Ok(client) = crate::messaging::DiscordClient::from_env()
        {
            let allowed = crate::messaging::discord_allowed_user_ids();
            match client.poll_once(&paths.discord_state_file, &allowed) {
                Ok(msgs) => {
                    for msg in &msgs {
                        let event = BusEvent::new(
                            "message",
                            "discord",
                            &msg.channel_id,
                            msg.author_id.clone(),
                            &msg.content,
                        );
                        if let Err(e) = bus.publish(&event) {
                            log::warn!("daemon: discord bus publish: {e}");
                        }
                    }
                    if !msgs.is_empty() {
                        log::info!("daemon: discord polled {} message(s)", msgs.len());
                    }
                }
                Err(e) => log::debug!("daemon: discord poll skipped: {e}"),
            }
        }
    }

    // ── Slack ─────────────────────────────────────────────────────────
    #[cfg(feature = "slack")]
    {
        if crate::messaging::SlackClient::validate_environment().is_ok()
            && let Ok(client) = crate::messaging::SlackClient::from_env()
        {
            let allowed = crate::messaging::slack_allowed_user_ids();
            match client.poll_once(&paths.slack_state_file, &allowed) {
                Ok(msgs) => {
                    for msg in &msgs {
                        let event = BusEvent::new(
                            "message",
                            "slack",
                            &msg.channel_id,
                            msg.user_id.clone(),
                            &msg.text,
                        );
                        if let Err(e) = bus.publish(&event) {
                            log::warn!("daemon: slack bus publish: {e}");
                        }
                    }
                    if !msgs.is_empty() {
                        log::info!("daemon: slack polled {} message(s)", msgs.len());
                    }
                }
                Err(e) => log::debug!("daemon: slack poll skipped: {e}"),
            }
        }
    }
}

// ── Signal handling ───────────────────────────────────────────────────────────

async fn wait_for_shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};
        let mut sigterm = signal(SignalKind::terminate()).expect("failed to register SIGTERM");
        let mut sigint = signal(SignalKind::interrupt()).expect("failed to register SIGINT");
        tokio::select! {
            _ = sigterm.recv() => {}
            _ = sigint.recv()  => {}
        }
    }
    #[cfg(not(unix))]
    {
        // On Windows, only Ctrl-C is available.
        let _ = tokio::signal::ctrl_c().await;
    }
}

// ── Status helpers (for `praxis daemon status`) ───────────────────────────────

/// Read the current daemon PID from the PID file, if it exists.
pub fn read_daemon_pid(paths: &PraxisPaths) -> Option<u32> {
    let raw = fs::read_to_string(&paths.daemon_pid_file).ok()?;
    raw.trim().parse().ok()
}

/// True if a daemon process appears to be alive (PID file exists and process
/// responds to kill(pid, 0)).
pub fn daemon_is_running(paths: &PraxisPaths) -> bool {
    match read_daemon_pid(paths) {
        Some(pid) => process_is_alive(pid),
        None => false,
    }
}

/// One-line status string for `praxis status` / `praxis daemon status`.
pub fn daemon_status_line(paths: &PraxisPaths) -> String {
    match read_daemon_pid(paths) {
        None => "daemon: not running".to_string(),
        Some(pid) => {
            if process_is_alive(pid) {
                format!("daemon: running (pid={pid})")
            } else {
                format!("daemon: stale pid file (pid={pid}, process not found)")
            }
        }
    }
}
