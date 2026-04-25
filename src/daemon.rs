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
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use anyhow::{Context, Result, bail};
use chrono::{DateTime, Utc};

use crate::{
    anatomy::refresh_stale_anatomy,
    backend::ConfiguredBackend,
    events::FileEventSink,
    heartbeat::write_heartbeat,
    identity::{IdentityPolicy, LocalIdentityPolicy, MarkdownGoalParser},
    lite::LiteMode,
    r#loop::{PraxisRuntime, RunOptions, RunSummary},
    paths::PraxisPaths,
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
    pub fn acquire(path: &Path, force: bool) -> Result<Self> {
        if path.exists() && !force {
            let raw = fs::read_to_string(path).unwrap_or_default();
            let existing_pid: u32 = raw.trim().parse().unwrap_or(0);
            if existing_pid > 0 && process_is_alive(existing_pid) {
                bail!(
                    "daemon already running (PID {existing_pid}).  \
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
        fs::write(path, pid.to_string())
            .with_context(|| format!("failed to write PID file {}", path.display()))?;
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
    // Set up a shared shutdown flag that signal handlers will flip.
    let shutdown = Arc::new(AtomicBool::new(false));

    // Spawn signal watchers that set the flag when SIGINT/SIGTERM arrives.
    {
        let flag = Arc::clone(&shutdown);
        tokio::spawn(async move {
            wait_for_shutdown_signal().await;
            log::info!("daemon: shutdown signal received — finishing current session then exiting");
            flag.store(true, Ordering::SeqCst);
        });
    }

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
    }

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
        if shutdown.load(Ordering::SeqCst) {
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

        let trigger = if let Some(intent) = crate::wakeup::consume_intent(&paths.data_dir)? {
            log::info!("daemon: wake intent from '{}': {}", intent.source, intent.reason);
            Some(SessionTrigger::WakeIntent { task: intent.task })
        } else if bus_watcher.has_new_events() {
            log::info!("daemon: new bus event(s) — triggering reactive session");
            Some(SessionTrigger::BusEvent)
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

            log::info!("daemon: starting session (trigger={label})");

            // Run the blocking session on the current thread (blocking_in_place
            // keeps the executor responsive for signal handling).
            let data_dir2 = data_dir.clone();
            let result = tokio::task::block_in_place(|| run_session_blocking(&data_dir2, task));

            match result {
                Ok(summary) => {
                    log::info!(
                        "daemon: session complete (trigger={label} outcome={} goal={:?})",
                        summary.outcome,
                        summary.selected_goal_title,
                    );
                    last_session_at = Some(Utc::now());
                    record_operator_activity(&paths, Utc::now());
                }
                Err(e) => {
                    log::error!("daemon: session failed (trigger={label}): {e:#}");
                    // Don't exit on session failure — stay alive and try again.
                    last_session_at = Some(Utc::now());
                }
            }

            // Re-check shutdown after a (potentially long) session.
            if shutdown.load(Ordering::SeqCst) {
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

    Ok(())
}

// ── Session runner ────────────────────────────────────────────────────────────

/// Load fresh config from disk and run a full Orient→Decide→Act→Reflect cycle.
///
/// Config is re-read every call so self-evolution changes (e.g., the agent
/// updated `praxis.toml` as part of the Act phase) take effect next session.
fn run_session_blocking(data_dir: &Path, task: Option<String>) -> Result<RunSummary> {
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
    tools.validate(&paths)?;

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
    };

    runtime.run_once(RunOptions {
        once: true, // Always single-pass from the daemon; the daemon manages the outer loop.
        force: task.is_some(),
        task,
    })
}

// ── Maintenance ───────────────────────────────────────────────────────────────

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
fn check_scheduled_jobs(paths: &PraxisPaths, now: DateTime<Utc>) -> Option<String> {
    use crate::tools::cron::ScheduledJobs;

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

    // Collect all due task descriptions.
    let tasks: Vec<String> = due.iter().map(|j| j.task.clone()).collect();
    let names: Vec<&str> = due.iter().map(|j| j.name.as_str()).collect();
    log::info!("daemon: scheduled jobs due: [{}]", names.join(", "));

    Some(tasks.join("; "))
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
