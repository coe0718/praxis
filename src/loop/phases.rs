use anyhow::{Context, Result};

use crate::{
    context::{ContextLoadRequest, LocalContextLoader, compact_if_needed, consume_compact, handoff},
    memory::{MemoryLinkStore, MemoryStore},
    state::SessionState,
    storage::{
        AnatomyStore, ApprovalStore, OperationalMemoryStore, ProviderUsageStore, QualityStore,
        SessionStore,
    },
    tools::{
        DEFAULT_LOOP_GUARD_LIMIT, GuardDecision, LoopGuard, SecurityPolicy, ToolRegistry,
        execute_request, sync_capabilities,
    },
    usage::{UsageBudgetMode, UsageBudgetPolicy},
};

use super::{
    AgentBackend, PraxisRuntime,
    planner::{GoalDecision, choose_goal},
};

impl<'a, B, C, E, G, I, S, T> PraxisRuntime<'a, B, C, E, G, I, S, T>
where
    B: AgentBackend,
    C: crate::time::Clock,
    E: crate::events::EventSink,
    G: crate::identity::GoalParser,
    I: crate::identity::IdentityPolicy,
    S: SessionStore
        + MemoryStore
        + MemoryLinkStore
        + ApprovalStore
        + QualityStore
        + ProviderUsageStore
        + OperationalMemoryStore
        + AnatomyStore,
    T: ToolRegistry,
{
    pub(super) fn orient(&self, state: &mut SessionState) -> Result<()> {
        // Consume any pending compaction request and open a clean context window.
        if let Some(req) = consume_compact(&self.paths.data_dir)? {
            let trigger = if req.trigger == crate::context::CompactionTrigger::Operator {
                "operator"
            } else {
                "auto"
            };
            self.emit(
                "agent:context_compacted",
                &format!("Context compacted ({trigger}). Opening clean context window."),
            )?;
        }

        self.identity.validate(self.paths)?;
        self.tools.validate(self.paths)?;
        let goals = self.goal_parser.load_goals(&self.paths.goals_file)?;
        let open_goals = goals
            .into_iter()
            .filter(|goal| !goal.completed)
            .collect::<Vec<_>>();
        let tool_summary = self.tools.summary(self.paths)?;
        let requested_task = state.requested_task.clone();
        let context = LocalContextLoader.load(
            self.store,
            ContextLoadRequest {
                config: self.config,
                paths: self.paths,
                state,
                tool_summary: &tool_summary,
                requested_task: requested_task.as_deref(),
                open_goals: &open_goals,
            },
        )?;
        state.context_sources = context
            .included_sources
            .iter()
            .map(|s| s.source.clone())
            .collect();

        // Context-rot prevention: write a handoff note when pressure exceeds 50%.
        let pressure = context.pressure_pct();
        handoff::write_if_needed(
            &self.paths.data_dir,
            pressure,
            state.selected_goal_id.as_deref(),
            state.action_summary.as_deref(),
            self.clock.now_utc(),
        )?;

        // Automatic compaction: schedule a fresh context window if pressure >= 80%.
        compact_if_needed(
            &self.paths.data_dir,
            pressure,
            state.selected_goal_id.as_deref(),
            self.clock.now_utc(),
        )?;

        state.orientation_summary = Some(format!(
            "Loaded {} open goals. {} Context pressure: {:.0}%. Repeated reads avoided: {}.",
            open_goals.len(),
            context.summary(),
            pressure * 100.0,
            state.repeated_reads_avoided,
        ));
        state.updated_at = self.clock.now_utc();
        Ok(())
    }

    pub(super) fn decide(&self, state: &mut SessionState) -> Result<()> {
        state.selected_tool_name = None;
        state.selected_tool_request_id = None;

        if let Some(task) = state.requested_task.clone() {
            if self.block_for_usage_budget(state, UsageBudgetMode::Run)? {
                return Ok(());
            }
            state.last_outcome = Some("task_selected".to_string());
            state.selected_goal_id = None;
            state.selected_goal_title = None;
            let output = self.backend.plan_action(None, Some(&task))?;
            state.provider_attempts.extend(output.attempts);
            state.action_summary = Some(output.summary);
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
        match choose_goal(&goals, &self.paths.data_dir, self.clock.now_utc())? {
            GoalDecision::Selected(goal) => {
                state.last_outcome = Some("goal_selected".to_string());
                state.selected_goal_id = Some(goal.id.clone());
                state.selected_goal_title = Some(goal.title.clone());
                if self.block_for_usage_budget(state, UsageBudgetMode::Run)? {
                    return Ok(());
                }
                let output = self.backend.plan_action(Some(&goal), None)?;
                state.provider_attempts.extend(output.attempts);
                state.action_summary = Some(output.summary);
            }
            GoalDecision::Waiting(summary) => {
                state.last_outcome = Some("waiting_on_dependencies".to_string());
                state.selected_goal_id = None;
                state.selected_goal_title = None;
                state.action_summary = Some(summary);
            }
            GoalDecision::Complete => {
                state.last_outcome = Some("stop_condition_met".to_string());
                state.selected_goal_id = None;
                state.selected_goal_title = None;
                state.action_summary =
                    Some("All current goals are complete. Stop condition reached.".to_string());
            }
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
        if self.block_for_usage_budget(state, UsageBudgetMode::Run)? {
            return Ok(());
        }
        let output = self.backend.finalize_action(
            &summary,
            super::reflect::selected_goal(state).as_ref(),
            state.requested_task.as_deref(),
        )?;
        state.provider_attempts.extend(output.attempts);
        state.action_summary = Some(output.summary);
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
        let invocation_key = invocation_key(&manifest, &request);

        match LoopGuard.record(state, &invocation_key, DEFAULT_LOOP_GUARD_LIMIT) {
            GuardDecision::Allow => {}
            GuardDecision::Blocked {
                consecutive_count,
                pattern_len,
            } => {
                let detail = if pattern_len > 1 {
                    format!("{pattern_len}-step pattern x{consecutive_count}")
                } else {
                    format!("{} x{}", manifest.name, consecutive_count)
                };
                self.emit("agent:loop_guard_triggered", &detail)?;
                state.last_outcome = Some("blocked_loop_guard".to_string());
                state.action_summary = Some(if pattern_len > 1 {
                    format!(
                        "Loop guard blocked a repeating {pattern_len}-step tool pattern after {consecutive_count} consecutive cycles."
                    )
                } else {
                    format!(
                        "Loop guard blocked tool {} after {} consecutive identical requests.",
                        manifest.name, consecutive_count
                    )
                });
                state.updated_at = self.clock.now_utc();
                return Ok(());
            }
        }

        let execution = execute_request(self.paths, &manifest, &request)?;
        self.store.mark_approval_consumed(request.id)?;
        sync_capabilities(self.tools, self.store, self.paths)?;
        self.emit(
            "agent:tool_call",
            &format!("{} {}", manifest.name, request.summary),
        )?;
        state.last_outcome = Some("tool_executed".to_string());
        state.action_summary = Some(execution.summary);
        state.updated_at = self.clock.now_utc();
        Ok(())
    }

    fn block_for_usage_budget(
        &self,
        state: &mut SessionState,
        mode: UsageBudgetMode,
    ) -> Result<bool> {
        let budgets = UsageBudgetPolicy::load_or_default(&self.paths.budgets_file)?;
        let decision = budgets
            .rule(mode)
            .check_attempts(&state.provider_attempts, mode);
        if !decision.blocked {
            return Ok(false);
        }
        state.last_outcome = Some("budget_exhausted".to_string());
        state.action_summary = Some(decision.summary);
        state.updated_at = self.clock.now_utc();
        self.emit(
            "agent:usage_budget_blocked",
            "Usage budget blocked another backend call.",
        )?;
        Ok(true)
    }
}

pub(super) fn invocation_key(
    manifest: &crate::tools::ToolManifest,
    request: &crate::storage::StoredApprovalRequest,
) -> String {
    format!(
        "{}|{}|{}|{}",
        manifest.name,
        request.summary,
        request.write_paths.join(","),
        request.payload_json.as_deref().unwrap_or("")
    )
}
