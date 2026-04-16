use anyhow::Result;

use crate::{
    config::AppConfig,
    events::EventSink,
    forensics::record_snapshot,
    heartbeat::write_heartbeat,
    hooks::{HookContext, HookRunner},
    identity::{GoalParser, IdentityPolicy},
    memory::{MemoryLinkStore, MemoryStore},
    paths::PraxisPaths,
    state::{SessionPhase, SessionState},
    storage::{
        AnatomyStore, ApprovalStore, DecisionReceiptStore, OperationalMemoryStore,
        ProviderUsageStore, QualityStore, SessionStore,
    },
    time::Clock,
    tools::ToolRegistry,
};

use super::{AgentBackend, RunOptions, RunSummary};

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

        while state.current_phase != SessionPhase::Sleep {
            self.run_phase(&mut state)?;
        }

        Ok(RunSummary {
            outcome: state
                .last_outcome
                .clone()
                .unwrap_or_else(|| "idle".to_string()),
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
        let ctx = HookContext::new(
            format!("phase.{phase_name}.start"),
            self.paths.data_dir.clone(),
        )
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
        record_snapshot(
            &self.paths.database_file,
            state,
            &format!("{event_kind}:start"),
        )?;
        handler(self, state)?;
        record_snapshot(
            &self.paths.database_file,
            state,
            &format!("{event_kind}:complete"),
        )?;

        // Observer hooks fire after the phase completes.
        let ctx_end = HookContext::new(
            format!("phase.{phase_name}.end"),
            self.paths.data_dir.clone(),
        )
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

        let now = self.clock.now_utc();
        let learning_store = crate::storage::SqliteSessionStore::new(self.paths.database_file.clone());
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

        if !already_ran_today && is_autonomous {
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

        Ok(())
    }
}
