use anyhow::Result;

use crate::{
    config::AppConfig,
    curator::Curator,
    events::EventSink,
    forensics::record_snapshot,
    heartbeat::write_heartbeat,
    hooks::{HookContext, HookRunner},
    identity::{GoalParser, IdentityPolicy},
    lite::LiteMode,
    memory::{MemoryLinkStore, MemoryStore},
    paths::PraxisPaths,
    plugins::PluginRegistry,
    state::{SessionPhase, SessionState},
    storage::{
        AnatomyStore, ApprovalStore, DecisionReceiptStore, OperationalMemoryStore,
        ProviderUsageStore, QualityStore, SessionStore,
    },
    time::Clock,
    tools::ToolRegistry,
};

use super::{AgentBackend, RunOptions, RunSummary};

/// (#50) Maximum depth of sub-agent spawning allowed.  A depth of 0 means
/// no spawning (the default for `Worker` agents).
const DEFAULT_MAX_SPAWN_DEPTH: u32 = 0;

pub struct PraxisRuntime<'a, B, C, E, G, I, S, T> {
    pub config: &'a AppConfig,
    pub paths: &'a PraxisPaths,
    pub backend: &'a B,
    pub clock: &'a C,
    pub events: &'a E,
    pub goal_parser: &'a G,
    pub identity: &'a I,
    pub store: &'a S,
    pub tools: &'a T,
    pub lite: &'a LiteMode,
    /// (#51) Tracks the timestamp of the last tool activity for inactivity
    /// timeout enforcement.  Updated after each phase completes.
    pub last_tool_activity: std::cell::Cell<Option<chrono::DateTime<chrono::Utc>>>,
    /// (#3) Loaded plugin registry — loaded once at session start.
    pub plugins: std::cell::RefCell<PluginRegistry>,
}

impl<'a, B, C, E, G, I, S, T> PraxisRuntime<'a, B, C, E, G, I, S, T>
where
    B: AgentBackend,
    C: Clock,
    E: EventSink,
    G: GoalParser,
    I: IdentityPolicy,
    S: SessionStore
        + MemoryStore
        + MemoryLinkStore
        + ApprovalStore
        + QualityStore
        + ProviderUsageStore
        + OperationalMemoryStore
        + AnatomyStore
        + DecisionReceiptStore,
    T: ToolRegistry,
{
    pub fn run_once(&self, options: RunOptions) -> Result<RunSummary> {
        let now = self.clock.now_utc();
        self.validate_options(&options)?;

        // ── Shell Hook: session.start ──────────────────────────────────────
        // Observer hooks fire at session start (non-blocking, fire-and-forget).
        let startup_hooks = HookRunner::from_paths(self.paths);
        let startup_ctx = HookContext::new("session.start", self.paths.data_dir.clone());
        startup_hooks.fire_observer("session.start", &startup_ctx, "*");

        // (#3) Load plugin registry for this session.
        let mut registry = PluginRegistry::new(self.paths);
        if let Err(e) = registry.load_all() {
            log::warn!("failed to load plugins: {e}");
        }
        *self.plugins.borrow_mut() = registry;

        // Consume any pending wake intent before the quiet-hours gate.
        // An urgent intent bypasses quiet hours; a normal intent respects them
        // but injects its task into the session.
        let (wake_bypasses_quiet, wake_task) =
            match crate::wakeup::consume_intent(&self.paths.data_dir)? {
                Some(intent) => {
                    let summary = crate::wakeup::format_summary(&intent);
                    self.emit("agent:wake_intent_consumed", &summary)?;
                    (intent.is_urgent(), intent.task)
                }
                None => (false, None),
            };

        let effective_task = options.task.or(wake_task);
        let force = options.force || wake_bypasses_quiet;

        if self.should_defer_for_quiet_hours(now, force)? {
            return self.defer_session(now, effective_task);
        }

        let mut state = self.load_or_create_state(now, effective_task)?;
        let resumed = state.resume_count > 0;
        write_heartbeat(
            &self.paths.heartbeat_file,
            "praxis",
            state.current_phase.to_string().as_str(),
            "Session loaded and ready to run.",
            now,
        )?;
        state.save(&self.paths.state_file)?;
        record_snapshot(&self.paths.database_file, &state, "session_loaded")?;

        // (#51) Initialize the inactivity tracker at session start.
        self.last_tool_activity.set(Some(now));

        while state.current_phase != SessionPhase::Sleep {
            // (#51) Check inactivity timeout before each phase.
            if let Some(inactive_summary) = self.check_inactivity_timeout(&state)? {
                return Ok(inactive_summary);
            }
            self.run_phase(&mut state)?;
            // (#51) Update last activity timestamp after each phase completes.
            self.last_tool_activity.set(Some(self.clock.now_utc()));
        }

        Ok(RunSummary {
            outcome: state.last_outcome.clone().unwrap_or_else(|| "idle".to_string()),
            phase: state.current_phase,
            resumed,
            selected_goal_id: state.selected_goal_id.clone(),
            selected_goal_title: state.selected_goal_title.clone(),
            selected_task: state.selected_task_label(),
            action_summary: state.action_summary.clone().unwrap_or_default(),
        })
    }

    fn run_phase(&self, state: &mut SessionState) -> Result<()> {
        match state.current_phase {
            SessionPhase::Orient => self.execute_phase(
                state,
                "agent:orient_start",
                "Loading identity, goals, tools, and local context.",
                Self::orient,
                SessionPhase::Decide,
            ),
            SessionPhase::Decide => self.execute_phase(
                state,
                "agent:decide_start",
                "Selecting the next unit of work.",
                Self::decide,
                SessionPhase::Act,
            ),
            SessionPhase::Act => self.execute_phase(
                state,
                "agent:act_start",
                "Executing safe internal maintenance or approved tool work.",
                Self::act,
                SessionPhase::Reflect,
            ),
            SessionPhase::Reflect => self.execute_reflect(state),
            SessionPhase::Sleep => Ok(()),
        }
    }

    fn execute_phase(
        &self,
        state: &mut SessionState,
        event_kind: &str,
        detail: &str,
        handler: fn(&Self, &mut SessionState) -> Result<()>,
        next_phase: SessionPhase,
    ) -> Result<()> {
        let phase_name = state.current_phase.to_string();
        let hooks = HookRunner::from_paths(self.paths);
        let ctx =
            HookContext::new(format!("phase.{phase_name}.start"), self.paths.data_dir.clone())
                .with_phase(&phase_name);

        // Interceptor hooks can abort a phase before it starts.
        hooks.fire_interceptor(&format!("phase.{phase_name}.start"), &ctx, "*")?;

        self.emit(event_kind, detail)?;
        write_heartbeat(
            &self.paths.heartbeat_file,
            "praxis",
            phase_name.as_str(),
            detail,
            self.clock.now_utc(),
        )?;
        state.save(&self.paths.state_file)?;
        record_snapshot(&self.paths.database_file, state, &format!("{event_kind}:start"))?;
        handler(self, state)?;
        record_snapshot(&self.paths.database_file, state, &format!("{event_kind}:complete"))?;

        // Observer hooks fire after the phase completes.
        let ctx_end =
            HookContext::new(format!("phase.{phase_name}.end"), self.paths.data_dir.clone())
                .with_phase(&phase_name);
        hooks.fire_observer(&format!("phase.{phase_name}.end"), &ctx_end, "*");

        state.mark_phase(next_phase, self.clock.now_utc());
        state.save(&self.paths.state_file)?;
        Ok(())
    }

    fn execute_reflect(&self, state: &mut SessionState) -> Result<()> {
        let hooks = HookRunner::from_paths(self.paths);
        let ctx = HookContext::new("session.start_reflect", self.paths.data_dir.clone())
            .with_phase("reflect");
        hooks.fire_interceptor("phase.reflect.start", &ctx, "*")?;

        self.emit("agent:reflect_start", "Recording the session outcome.")?;
        write_heartbeat(
            &self.paths.heartbeat_file,
            "praxis",
            state.current_phase.to_string().as_str(),
            "Recording the session outcome.",
            self.clock.now_utc(),
        )?;
        state.save(&self.paths.state_file)?;
        record_snapshot(&self.paths.database_file, state, "agent:reflect_start")?;
        self.reflect(state)?;
        let decayed = self.store.decay_cold_memories(self.clock.now_utc())?;
        let consolidation = self.store.consolidate_memories(self.clock.now_utc());
        match consolidation {
            Ok(ref summary) if summary.consolidated > 0 || summary.pruned > 0 => {
                if let Err(e) = self.emit(
                    "agent:memory_consolidated",
                    &format!(
                        "{} hot clusters promoted to cold, {} dead cold memories pruned.",
                        summary.consolidated, summary.pruned
                    ),
                ) {
                    log::warn!("failed to emit memory consolidation event: {e}");
                }
            }
            Err(e) => log::warn!("memory consolidation failed: {e}"),
            _ => {}
        }

        let now = self.clock.now_utc();
        let learning_store =
            crate::storage::SqliteSessionStore::new(self.paths.database_file.clone());
        let already_ran_today = learning_store
            .latest_learning_run()
            .ok()
            .flatten()
            .and_then(|run| chrono::DateTime::parse_from_rfc3339(&run.completed_at).ok())
            .map(|ts| ts.date_naive() == now.date_naive())
            .unwrap_or(false);

        // Only auto-run learning for autonomous (non-operator-driven) sessions.
        // Operator-injected tasks run learning on demand via `praxis learn run`.
        let is_autonomous = state.requested_task.is_none();

        if !already_ran_today
            && is_autonomous
            && !self.lite.skip_capability(crate::lite::LiteCapability::Learning)
        {
            match crate::learning::run_once(self.paths, &learning_store, now) {
                Ok(summary) if summary.opportunities_created > 0 => {
                    self.emit(
                        "agent:learning_opportunities_found",
                        &format!(
                            "{} new learning opportunities queued.",
                            summary.opportunities_created
                        ),
                    )?;
                }
                Err(e) => log::warn!("learning run failed: {e}"),
                _ => {}
            }
        }

        // (#6) Autonomous curator: run skill grading cycle if due.
        // The curator evaluates skills by usage (40%), age (20%), quality (20%),
        // and dependencies (20%) on a 7-day schedule.  Results are written to
        // curator reports in the data directory.
        if !self.lite.skip_capability(crate::lite::LiteCapability::Curator) {
            let curator_config = crate::curator::CuratorConfig::default();
            let curator = Curator::new(curator_config, self.paths);
            match curator.is_cycle_due() {
                Ok(true) => match curator.run_cycle() {
                    Ok(report) => {
                        curator.mark_cycle_run()?;
                        if let Err(e) = self.emit(
                            "agent:curator_cycle_complete",
                            &format!(
                                "curator: graded {} skills, {} prune candidates, {} promote candidates",
                                report.total_skills,
                                report.prune_candidates.len(),
                                report.promote_candidates.len(),
                            ),
                        ) {
                            log::warn!("failed to emit curator event: {e}");
                        }
                    }
                    Err(e) => log::warn!("curator cycle failed: {e}"),
                },
                Ok(false) => {} // not due yet
                Err(e) => log::warn!("curator cycle check failed: {e}"),
            }
        }

        if !self.lite.skip_capability(crate::lite::LiteCapability::Brief) {
            try_send_morning_brief(self.paths, now);
        }
        if decayed > 0 {
            self.emit(
                "agent:cold_memory_decayed",
                &format!("{decayed} stale cold memories decayed in place."),
            )?;
        }
        write_heartbeat(
            &self.paths.heartbeat_file,
            "praxis",
            "sleep",
            "Session completed and returned to sleep.",
            self.clock.now_utc(),
        )?;
        record_snapshot(&self.paths.database_file, state, "agent:reflect_complete")?;
        state.save(&self.paths.state_file)?;

        // session.end observer hooks — fire after all state is persisted.
        let ctx_end = HookContext::new("session.end", self.paths.data_dir.clone())
            .with_outcome(state.last_outcome.clone().unwrap_or_else(|| "idle".to_string()));
        hooks.fire_observer("session.end", &ctx_end, "*");

        // Commit all state changes to the data-dir git repo (if one exists).
        crate::cli::git::auto_commit(self.paths);

        Ok(())
    }

    /// (#51) Check whether the session has exceeded the inactivity timeout.
    /// Returns `Some(RunSummary)` if the session should be ended gracefully,
    /// or `None` if the session should continue.
    fn check_inactivity_timeout(&self, state: &SessionState) -> Result<Option<RunSummary>> {
        let timeout_secs = match self.config.agent.inactivity_timeout_secs {
            Some(secs) => secs,
            None => return Ok(None),
        };

        let last = match self.last_tool_activity.get() {
            Some(ts) => ts,
            None => return Ok(None),
        };

        let now = self.clock.now_utc();
        let elapsed = (now - last).num_seconds().max(0) as u64;

        if elapsed >= timeout_secs {
            log::info!(
                "session exceeded inactivity timeout of {timeout_secs}s \
                 (last activity {elapsed}s ago) — ending session gracefully"
            );
            self.emit(
                "agent:inactivity_timeout",
                &format!("Session ended due to inactivity ({elapsed}s > {timeout_secs}s timeout)."),
            )?;

            // Gracefully end the session — mark as sleep.
            return Ok(Some(RunSummary {
                outcome: "inactivity_timeout".to_string(),
                phase: SessionPhase::Sleep,
                resumed: state.resume_count > 0,
                selected_goal_id: state.selected_goal_id.clone(),
                selected_goal_title: state.selected_goal_title.clone(),
                selected_task: state.selected_task_label(),
                action_summary: format!(
                    "Session ended after {elapsed}s of inactivity (timeout: {timeout_secs}s)."
                ),
            }));
        }

        Ok(None)
    }
}

/// (#50) Check whether the agent is allowed to spawn a sub-agent at the given
/// depth.  Returns `Ok(())` if allowed, or an error describing why not.
///
/// This is a standalone function so it can be called from any phase without
/// needing the full runtime generic context.
pub fn check_spawn_depth(config: &AppConfig, current_depth: u32) -> Result<()> {
    use crate::config::model::AgentRole;

    if config.agent.disable_sub_agents {
        anyhow::bail!("sub-agent spawning is disabled for this agent");
    }
    if config.agent.role == AgentRole::Worker {
        anyhow::bail!("agent role is 'worker' — only orchestrators can spawn sub-agents");
    }
    let max_depth = if config.agent.max_spawn_depth > 0 {
        config.agent.max_spawn_depth
    } else {
        DEFAULT_MAX_SPAWN_DEPTH
    };
    if current_depth >= max_depth {
        anyhow::bail!("spawn depth {} exceeds maximum allowed depth {}", current_depth, max_depth);
    }
    Ok(())
}

fn try_send_morning_brief(paths: &crate::paths::PraxisPaths, now: chrono::DateTime<chrono::Utc>) {
    let brief_sent_path = paths.data_dir.join("brief_sent.txt");
    let today = now.date_naive().to_string();

    let already_sent = std::fs::read_to_string(&brief_sent_path)
        .map(|s| s.trim() == today)
        .unwrap_or(false);
    if already_sent {
        return;
    }

    let bot = match crate::messaging::TelegramBot::from_env() {
        Ok(b) => b,
        Err(_) => return,
    };
    let Some(chat_id) = bot.primary_chat_id() else {
        return;
    };

    let brief = match crate::brief::generate_brief(paths, now) {
        Ok(b) => b,
        Err(e) => {
            log::warn!("brief generation failed: {e}");
            return;
        }
    };

    // Write the guard file BEFORE sending so a crash or send failure
    // cannot cause a duplicate brief later in the same day.
    if let Err(e) = std::fs::write(&brief_sent_path, &today) {
        log::warn!("failed to record brief_sent date: {e}");
        return;
    }

    if let Err(e) = bot.send_message(chat_id, &brief) {
        log::warn!("brief send failed: {e}");
    }
}
