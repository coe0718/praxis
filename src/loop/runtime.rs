use anyhow::Result;

use crate::{
    config::AppConfig,
    events::EventSink,
    forensics::record_snapshot,
    heartbeat::write_heartbeat,
    identity::{GoalParser, IdentityPolicy},
    memory::MemoryStore,
    paths::PraxisPaths,
    state::{SessionPhase, SessionState},
    storage::{
        AnatomyStore, ApprovalStore, OperationalMemoryStore, ProviderUsageStore, QualityStore,
        SessionStore,
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
        + ApprovalStore
        + QualityStore
        + ProviderUsageStore
        + OperationalMemoryStore
        + AnatomyStore,
    T: ToolRegistry,
{
    pub fn run_once(&self, options: RunOptions) -> Result<RunSummary> {
        let now = self.clock.now_utc();
        self.validate_options(&options)?;

        if self.should_defer_for_quiet_hours(now, options.force)? {
            return self.defer_session(now, options.task);
        }

        let mut state = self.load_or_create_state(now, options.task)?;
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
        self.emit(event_kind, detail)?;
        write_heartbeat(
            &self.paths.heartbeat_file,
            "praxis",
            state.current_phase.to_string().as_str(),
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
        state.mark_phase(next_phase, self.clock.now_utc());
        state.save(&self.paths.state_file)?;
        Ok(())
    }

    fn execute_reflect(&self, state: &mut SessionState) -> Result<()> {
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
        Ok(())
    }
}
