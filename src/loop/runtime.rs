use anyhow::Result;

use crate::{
    config::AppConfig,
    context::LocalContextLoader,
    events::EventSink,
    identity::{GoalParser, IdentityPolicy},
    memory::{MemoryStore, NewHotMemory},
    paths::PraxisPaths,
    state::{SessionPhase, SessionState},
    storage::{SessionRecord, SessionStore},
    time::Clock,
};

use super::{AgentBackend, RunOptions, RunSummary};

pub struct PraxisRuntime<'a, B, C, E, G, I, S> {
    pub config: &'a AppConfig,
    pub paths: &'a PraxisPaths,
    pub backend: &'a B,
    pub clock: &'a C,
    pub events: &'a E,
    pub goal_parser: &'a G,
    pub identity: &'a I,
    pub store: &'a S,
}

impl<'a, B, C, E, G, I, S> PraxisRuntime<'a, B, C, E, G, I, S>
where
    B: AgentBackend,
    C: Clock,
    E: EventSink,
    G: GoalParser,
    I: IdentityPolicy,
    S: SessionStore + MemoryStore,
{
    pub fn run_once(&self, options: RunOptions) -> Result<RunSummary> {
        let now = self.clock.now_utc();
        self.validate_options(&options)?;

        if self.should_defer_for_quiet_hours(now, options.force)? {
            return self.defer_session(now, options.task);
        }

        let mut state = self.load_or_create_state(now, options.task)?;
        let resumed = state.resume_count > 0;
        state.save(&self.paths.state_file)?;

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
            selected_task: state.requested_task.clone(),
            action_summary: state.action_summary.clone().unwrap_or_default(),
        })
    }

    fn run_phase(&self, state: &mut SessionState) -> Result<()> {
        match state.current_phase {
            SessionPhase::Orient => self.execute_phase(
                state,
                "agent:orient_start",
                "Loading identity, goals, and local context.",
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
                "Executing safe internal maintenance.",
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
        state.save(&self.paths.state_file)?;
        handler(self, state)?;
        state.mark_phase(next_phase, self.clock.now_utc());
        state.save(&self.paths.state_file)?;
        Ok(())
    }

    fn execute_reflect(&self, state: &mut SessionState) -> Result<()> {
        self.emit("agent:reflect_start", "Recording the session outcome.")?;
        state.save(&self.paths.state_file)?;
        self.reflect(state)?;
        state.save(&self.paths.state_file)?;
        Ok(())
    }

    fn orient(&self, state: &mut SessionState) -> Result<()> {
        self.identity.validate(self.paths)?;
        let goals = self.goal_parser.load_goals(&self.paths.goals_file)?;
        let open_goals = goals
            .into_iter()
            .filter(|goal| !goal.completed)
            .collect::<Vec<_>>();
        let context = LocalContextLoader.load(
            self.config,
            self.paths,
            self.store,
            state.requested_task.as_deref(),
            &open_goals,
        )?;
        state.orientation_summary = Some(format!(
            "Loaded {} open goals. {}",
            open_goals.len(),
            context.summary(),
        ));
        state.updated_at = self.clock.now_utc();
        Ok(())
    }

    fn decide(&self, state: &mut SessionState) -> Result<()> {
        if let Some(task) = state.requested_task.as_deref() {
            state.last_outcome = Some("task_selected".to_string());
            state.selected_goal_id = None;
            state.selected_goal_title = None;
            state.action_summary = Some(self.backend.plan_action(None, Some(task))?);
            state.updated_at = self.clock.now_utc();
            return Ok(());
        }

        let goals = self.goal_parser.load_goals(&self.paths.goals_file)?;
        if let Some(goal) = goals.into_iter().find(|goal| !goal.completed) {
            state.last_outcome = Some("goal_selected".to_string());
            state.selected_goal_id = Some(goal.id.clone());
            state.selected_goal_title = Some(goal.title.clone());
            state.action_summary = Some(self.backend.plan_action(Some(&goal), None)?);
        } else {
            state.last_outcome = Some("idle".to_string());
            state.selected_goal_id = None;
            state.selected_goal_title = None;
            state.action_summary = Some(self.backend.plan_action(None, None)?);
        }

        state.updated_at = self.clock.now_utc();
        Ok(())
    }

    fn act(&self, state: &mut SessionState) -> Result<()> {
        let summary = state
            .action_summary
            .clone()
            .unwrap_or_else(|| "No action was selected.".to_string());
        state.action_summary = Some(format!(
            "{summary} Act phase completed without external side effects."
        ));
        state.updated_at = self.clock.now_utc();
        Ok(())
    }

    fn reflect(&self, state: &mut SessionState) -> Result<()> {
        let ended_at = self.clock.now_utc();
        let outcome = state
            .last_outcome
            .clone()
            .unwrap_or_else(|| "idle".to_string());
        let record = SessionRecord {
            day: self.identity.read_day_count(self.paths)?,
            started_at: state.started_at,
            ended_at,
            outcome: outcome.clone(),
            selected_goal_id: state.selected_goal_id.clone(),
            selected_goal_title: state.selected_goal_title.clone(),
            selected_task: state.requested_task.clone(),
            action_summary: state.action_summary.clone().unwrap_or_default(),
            phase_durations_json: serde_json::json!({
                "orient": 0,
                "decide": 0,
                "act": 0,
                "reflect": 0
            })
            .to_string(),
        };

        let stored = self.store.record_session(&record)?;
        let memory_summary = format!(
            "Session outcome {} with summary: {}",
            outcome, stored.action_summary
        );
        self.store.insert_hot_memory(NewHotMemory {
            content: memory_summary,
            summary: Some(outcome),
            importance: 0.7,
            tags: vec!["session".to_string(), "foundation".to_string()],
            expires_at: None,
        })?;
        self.identity.append_journal(self.paths, &stored)?;
        self.identity.append_metrics(self.paths, &stored)?;
        self.emit("agent:goal_complete", &stored.outcome)?;

        state.finish(stored.outcome, ended_at);
        state.updated_at = ended_at;
        Ok(())
    }
}
