use anyhow::{Context, Result};

use crate::{
    context::LocalContextLoader,
    memory::MemoryStore,
    state::SessionState,
    storage::{ApprovalStore, QualityStore, SessionStore},
    tools::{DEFAULT_LOOP_GUARD_LIMIT, GuardDecision, LoopGuard, SecurityPolicy, ToolRegistry},
};

use super::{AgentBackend, PraxisRuntime};

impl<'a, B, C, E, G, I, S, T> PraxisRuntime<'a, B, C, E, G, I, S, T>
where
    B: AgentBackend,
    C: crate::time::Clock,
    E: crate::events::EventSink,
    G: crate::identity::GoalParser,
    I: crate::identity::IdentityPolicy,
    S: SessionStore + MemoryStore + ApprovalStore + QualityStore,
    T: ToolRegistry,
{
    pub(super) fn orient(&self, state: &mut SessionState) -> Result<()> {
        self.identity.validate(self.paths)?;
        self.tools.validate(self.paths)?;
        let goals = self.goal_parser.load_goals(&self.paths.goals_file)?;
        let open_goals = goals
            .into_iter()
            .filter(|goal| !goal.completed)
            .collect::<Vec<_>>();
        let tool_summary = self.tools.summary(self.paths)?;
        let context = LocalContextLoader.load(
            self.config,
            self.paths,
            self.store,
            &tool_summary,
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

    pub(super) fn decide(&self, state: &mut SessionState) -> Result<()> {
        state.selected_tool_name = None;
        state.selected_tool_request_id = None;

        if let Some(task) = state.requested_task.as_deref() {
            state.last_outcome = Some("task_selected".to_string());
            state.selected_goal_id = None;
            state.selected_goal_title = None;
            state.action_summary = Some(self.backend.plan_action(None, Some(task))?);
            state.updated_at = self.clock.now_utc();
            return Ok(());
        }

        if let Some(request) = self.store.next_approved_request()? {
            state.last_outcome = Some("approved_tool_selected".to_string());
            state.selected_tool_name = Some(request.tool_name.clone());
            state.selected_tool_request_id = Some(request.id);
            state.selected_goal_id = None;
            state.selected_goal_title = None;
            state.action_summary = Some(format!(
                "Approved tool request #{} queued for execution: {}",
                request.id, request.summary
            ));
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

    pub(super) fn act(&self, state: &mut SessionState) -> Result<()> {
        if let Some(request_id) = state.selected_tool_request_id {
            return self.execute_tool_request(state, request_id);
        }

        let summary = state
            .action_summary
            .clone()
            .unwrap_or_else(|| "No action was selected.".to_string());
        state.action_summary = Some(self.backend.finalize_action(
            &summary,
            super::reflect::selected_goal(state).as_ref(),
            state.requested_task.as_deref(),
        )?);
        state.updated_at = self.clock.now_utc();
        Ok(())
    }

    fn execute_tool_request(&self, state: &mut SessionState, request_id: i64) -> Result<()> {
        let request = self
            .store
            .get_approval(request_id)?
            .with_context(|| format!("tool request {request_id} is missing"))?;
        let manifest = self
            .tools
            .get(self.paths, &request.tool_name)?
            .with_context(|| format!("tool manifest {} is missing", request.tool_name))?;

        SecurityPolicy.validate_request(self.config, self.paths, &manifest, &request)?;
        let invocation_key = format!(
            "{}|{}|{}",
            manifest.name,
            request.summary,
            request.write_paths.join(",")
        );

        match LoopGuard.record(state, &invocation_key, DEFAULT_LOOP_GUARD_LIMIT) {
            GuardDecision::Allow => {}
            GuardDecision::Blocked { consecutive_count } => {
                self.emit(
                    "agent:loop_guard_triggered",
                    &format!("{} x{}", manifest.name, consecutive_count),
                )?;
                state.last_outcome = Some("blocked_loop_guard".to_string());
                state.action_summary = Some(format!(
                    "Loop guard blocked tool {} after {} consecutive identical requests.",
                    manifest.name, consecutive_count
                ));
                state.updated_at = self.clock.now_utc();
                return Ok(());
            }
        }

        self.store.mark_approval_consumed(request.id)?;
        self.emit(
            "agent:tool_call",
            &format!("{} {}", manifest.name, request.summary),
        )?;

        let rehearsal = if manifest.rehearsal_required {
            "Rehearsal required; "
        } else {
            ""
        };
        state.last_outcome = Some("tool_executed".to_string());
        state.action_summary = Some(format!(
            "{rehearsal}Stub execution recorded for approved tool {} with {} declared write paths. No external side effects were performed in milestone 3.",
            manifest.name,
            request.write_paths.len()
        ));
        state.updated_at = self.clock.now_utc();
        Ok(())
    }
}
