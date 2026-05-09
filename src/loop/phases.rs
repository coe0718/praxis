use anyhow::{Context, Result};

use crate::{
    context::{
        ContextLoadRequest, LocalContextLoader, compact_if_needed, consume_compact, handoff,
    },
    hands::HandStore,
    hooks::{ApprovalVerdict, HookContext, HookRunner},
    memory::{MemoryLinkStore, MemoryStore},
    paths::PraxisPaths,
    speculative::{SpeculativeBranch, select_branch},
    state::SessionState,
    storage::{
        AnatomyStore, ApprovalStatus, ApprovalStore, DecisionReceiptStore, NewApprovalRequest,
        NewDecisionReceipt, OperationalMemoryStore, ProviderUsageStore, QualityStore, SessionStore,
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
        + AnatomyStore
        + DecisionReceiptStore,
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

        // OpenMolt integration registry: register available providers as tool awareness
        // so the agent knows which services are natively integrated.
        if !self.lite.skip_capability(crate::lite::LiteCapability::OpenMolt) {
            let summary = crate::openmolt::register_integrations();
            log::debug!("orient: {summary}");
        }

        enforce_active_hand(self.paths, self.tools)?;

        // Convert any inbound delegation tasks into approval requests so the
        // operator reviews them before they run.
        match crate::delegation::drain_inbound_delegation(&self.paths.delegation_queue_file) {
            Ok(tasks) if !tasks.is_empty() => {
                for task in &tasks {
                    let req = NewApprovalRequest {
                        tool_name: "shell-exec".to_string(),
                        summary: format!("[delegated from {}] {}", task.source, task.task),
                        requested_by: task
                            .link_name
                            .clone()
                            .unwrap_or_else(|| "delegation".to_string()),
                        write_paths: Vec::new(),
                        payload_json: None,
                        status: crate::storage::ApprovalStatus::Pending,
                    };
                    if let Err(e) = self.store.queue_approval(&req) {
                        log::warn!("failed to queue delegated task as approval: {e}");
                    } else {
                        // Notify operator with approve/deny buttons.
                        if let Some(stored) = self
                            .store
                            .list_approvals(Some(crate::storage::ApprovalStatus::Pending))?
                            .last()
                        {
                            notify_approval_request(stored.id, &req.tool_name, &req.summary);
                        }
                    }
                }
                if let Err(e) = self.emit(
                    "agent:delegation_received",
                    &format!("{} inbound delegation task(s) queued for approval.", tasks.len()),
                ) {
                    log::warn!("failed to emit delegation event: {e}");
                }
            }
            Err(e) => log::warn!("failed to drain delegation queue: {e}"),
            _ => {}
        }

        if let Err(e) = crate::anatomy::refresh_stale_anatomy(self.paths) {
            log::warn!("anatomy refresh failed: {e}");
        }

        let goals = self.goal_parser.load_goals(&self.paths.goals_file)?;
        let open_goals = goals.into_iter().filter(|goal| !goal.completed).collect::<Vec<_>>();
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
        state.context_sources = context.included_sources.iter().map(|s| s.source.clone()).collect();
        state.rendered_context = Some(context.render());

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
            let output =
                self.backend.plan_action(None, Some(&task), state.rendered_context.as_deref())?;
            state.provider_attempts.extend(output.attempts);
            state.action_summary = Some(output.summary);
            state.updated_at = self.clock.now_utc();
            return Ok(());
        }

        if let Some(request) = self.store.next_approved_request()? {
            // ── Shell Hook: approval.before ────────────────────────────────
            // Allow approval hooks to auto-approve, reject, or defer the request.
            let hooks = HookRunner::from_paths(self.paths);
            let approval_ctx = HookContext::new("approval.before", self.paths.data_dir.clone())
                .with_phase("decide")
                .with_tool(&request.tool_name, Some(request.id));
            let verdict = hooks.fire_approval_hooks(
                &request.tool_name,
                &approval_ctx,
                request.payload_json.as_deref(),
            );
            match verdict {
                ApprovalVerdict::Approve => {
                    // Hook approved — proceed as normal (already approved in queue).
                    log::info!(
                        "hooks: approval hook auto-approved tool '{}' (request #{})",
                        request.tool_name,
                        request.id
                    );
                }
                ApprovalVerdict::Reject(note) => {
                    // Hook rejected — mark the request as rejected.
                    log::info!(
                        "hooks: approval hook rejected tool '{}' (request #{}): {note}",
                        request.tool_name,
                        request.id
                    );
                    self.store.set_approval_status(
                        request.id,
                        ApprovalStatus::Rejected,
                        Some(&format!("auto-rejected by approval hook: {note}")),
                    )?;
                    state.last_outcome = Some("hook_rejected_tool".to_string());
                    state.action_summary = Some(format!(
                        "Approval hook rejected tool '{}' (request #{}): {note}",
                        request.tool_name, request.id
                    ));
                    state.updated_at = self.clock.now_utc();
                    return Ok(());
                }
                ApprovalVerdict::Defer => {
                    // No opinion — proceed with normal approved flow.
                }
            }

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
                let output = self.backend.plan_action(
                    Some(&goal),
                    None,
                    state.rendered_context.as_deref(),
                )?;
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

        // ── Marketplace integration ───────────────────────────────────────────────
        // Check for available paid work when no local goals match
        if state.selected_goal_id.is_none()
            && state.last_outcome.as_deref() == Some("waiting_on_dependencies")
        {
            let client = crate::marketplace::MarketplaceClient::new("praxis");
            let work_items = client.query_work(None);
            if !work_items.is_empty() {
                // Create a temporary goal from marketplace work
                let work = &work_items[0];
                state.last_outcome = Some("marketplace_work_found".to_string());
                state.selected_goal_id = Some(format!("market-{}", work.id));
                state.selected_goal_title = Some(format!("[Marketplace] {}", work.title));
                state.action_summary = Some(format!(
                    "Found marketplace work: {} (max_price: {} wei)",
                    work.title, work.max_price
                ));
            }
        }

        state.updated_at = self.clock.now_utc();
        self.write_decision_receipt(state)?;
        Ok(())
    }

    fn write_decision_receipt(&self, state: &SessionState) -> anyhow::Result<()> {
        let reason_code = match state.last_outcome.as_deref() {
            Some(code) => code.to_string(),
            None => return Ok(()),
        };
        let confidence = decision_confidence(&reason_code);
        let approval_required = state.selected_tool_request_id.is_some();
        let receipt = NewDecisionReceipt {
            session_started_at: state.started_at,
            reason_code,
            goal_id: state.selected_goal_id.clone(),
            chosen_action: state
                .action_summary
                .clone()
                .unwrap_or_else(|| "No action selected.".to_string()),
            context_sources: state.context_sources.clone(),
            confidence,
            approval_required,
        };
        self.store.record_decision(&receipt)?;
        Ok(())
    }

    pub(super) fn act(&self, state: &mut SessionState) -> Result<()> {
        if let Some(request_id) = state.selected_tool_request_id {
            return self.execute_tool_request(state, request_id);
        }

        // Mid-session steering: a wake intent written after the session started
        // redirects the current action without an LLM call.
        if let Ok(Some(steer)) = crate::wakeup::consume_intent(&self.paths.data_dir)
            && let Some(task) = steer.task
        {
            self.emit(
                "agent:steered",
                &format!("mid-session redirect from {}: {task}", steer.source),
            )?;
            state.last_outcome = Some("steered".to_string());
            state.action_summary = Some(format!(
                "Session redirected by steering signal from {}: {task}",
                steer.source
            ));
            state.updated_at = self.clock.now_utc();
            return Ok(());
        }

        // Outbound delegation: if an enabled link can carry this task, send it
        // to the remote agent and mark the session as delegated.
        let summary = state
            .action_summary
            .clone()
            .unwrap_or_else(|| "No action was selected.".to_string());
        let task_key = state.selected_goal_title.as_deref().unwrap_or(summary.as_str());
        if let Some(delegated_to) =
            self.try_delegate(state, task_key, &summary, self.clock.now_utc())?
        {
            state.last_outcome = Some("delegated".to_string());
            state.action_summary = Some(delegated_to.clone());
            // Channel notification: delegated task alert.
            crate::channels::notify_event("delegated", &format!("{task_key}: {delegated_to}"));
            state.updated_at = self.clock.now_utc();
            return Ok(());
        }

        // Agent Federation: decompose complex tasks across specialized agent roles.
        if !self.lite.skip_capability(crate::lite::LiteCapability::Federation)
            && let Some(goal_title) = state.selected_goal_title.as_deref()
            && summary.len() > 200
            && let Ok(fed) = self.try_federation(goal_title, &summary)
        {
            state.last_outcome = Some("federated".to_string());
            state.action_summary = Some(fed);
            crate::channels::notify_event("federated", goal_title);
            state.updated_at = self.clock.now_utc();
            return Ok(());
        }

        if self.block_for_usage_budget(state, UsageBudgetMode::Run)? {
            return Ok(());
        }

        // Speculative execution: rehearse an alternative approach and pick the
        // higher-scoring branch before committing to finalize_action.
        let summary = if self.lite.skip_capability(crate::lite::LiteCapability::Speculative) {
            summary.clone()
        } else {
            self.run_speculative(&summary, state)?
        };

        // Wave execution: log availability for parallel tool plans when enabled.
        if !self.lite.skip_capability(crate::lite::LiteCapability::Wave) {
            log::debug!("wave: parallel execution engine available for multi-tool plans");
        }

        let output = self.backend.finalize_action(
            &summary,
            super::reflect::selected_goal(state).as_ref(),
            state.requested_task.as_deref(),
            state.rendered_context.as_deref(),
        )?;
        state.provider_attempts.extend(output.attempts);
        state.action_summary = Some(output.summary);
        state.updated_at = self.clock.now_utc();
        Ok(())
    }

    /// Generate a conservative alternative plan branch and return whichever
    /// branch scores higher against the current goal/task keywords.
    fn run_speculative(&self, primary_summary: &str, state: &mut SessionState) -> Result<String> {
        let alt_context = state.rendered_context.as_deref().map(|ctx| {
            format!(
                "{ctx}\n\n[Speculative branch: consider a more conservative, reversible approach.]"
            )
        });

        let goal = super::reflect::selected_goal(state);
        let alt_output = match self.backend.plan_action(
            goal.as_ref(),
            state.requested_task.as_deref(),
            alt_context.as_deref(),
        ) {
            Ok(o) => o,
            Err(e) => {
                log::warn!("speculative branch-b generation failed: {e}");
                return Ok(primary_summary.to_string());
            }
        };
        state.provider_attempts.extend(alt_output.attempts);

        let branch_a = SpeculativeBranch::new("branch-a", "primary approach", primary_summary);
        let branch_b =
            SpeculativeBranch::new("branch-b", "conservative alternative", &alt_output.summary);

        let success_criteria = speculative_keywords(
            state
                .selected_goal_title
                .as_deref()
                .or(state.requested_task.as_deref())
                .unwrap_or(""),
        );
        let trust_constraints = vec![
            "force push".to_string(),
            "delete production".to_string(),
            "drop table".to_string(),
            "rm -rf".to_string(),
            "truncate".to_string(),
            "--no-verify".to_string(),
            "hard reset".to_string(),
            "--force".to_string(),
        ];

        let Some(result) =
            select_branch(vec![branch_a, branch_b], &success_criteria, &trust_constraints)
        else {
            return Ok(primary_summary.to_string());
        };

        if let Err(e) = self.emit("agent:speculative_branch_selected", &result.rationale) {
            log::warn!("failed to emit speculative event: {e}");
        }

        Ok(result.winner.plan_text)
    }

    fn try_delegate(
        &self,
        _state: &SessionState,
        task_key: &str,
        task_summary: &str,
        now: chrono::DateTime<chrono::Utc>,
    ) -> Result<Option<String>> {
        let mut store =
            crate::delegation::DelegationStore::load(&self.paths.delegation_links_file)?;
        let available: Vec<String> =
            store.available_outbound(task_key).into_iter().map(|l| l.name.clone()).collect();
        let Some(link_name) = available.into_iter().next() else {
            // No local delegation link — try A2A protocol if a remote agent URL is configured.
            if let Ok(agent_url) = std::env::var("PRAXIS_A2A_AGENT_URL")
                && let Ok(client) = crate::a2a::A2aClient::new(&agent_url)
            {
                let req = crate::a2a::types::SendTaskRequest {
                    id: format!("praxis_{}", now.timestamp_millis()),
                    session_id: format!("session_{}", now.timestamp_millis()),
                    message: crate::a2a::types::Message {
                        role: "user".to_string(),
                        parts: vec![crate::a2a::types::Part::Text {
                            text: task_summary.to_string(),
                        }],
                    },
                };
                match client.send_task(&req) {
                    Ok(resp) => {
                        self.emit(
                            "agent:a2a_delegated",
                            &format!(
                                "task sent via A2A to {} — status: {:?}",
                                agent_url, resp.status
                            ),
                        )?;
                        return Ok(Some(format!(
                            "Task delegated via A2A to {agent_url}: {task_summary}"
                        )));
                    }
                    Err(e) => {
                        log::warn!("A2A delegation failed: {e}");
                        return Ok(None);
                    }
                }
            }
            return Ok(None);
        };
        if let Some(link) = store.links.get_mut(&link_name) {
            crate::delegation::send_over_link(link, task_summary, "praxis", now)?;
        }
        // Persist before acquiring so a save failure does not leave the
        // in-memory state inconsistent with the on-disk store.
        store.save(&self.paths.delegation_links_file)?;
        store.acquire(&link_name);
        let endpoint =
            store.links.get(&link_name).map(|l| l.endpoint.as_str()).unwrap_or("unknown");
        self.emit(
            "agent:delegated",
            &format!("task delegated to {link_name} ({endpoint}): {task_summary}"),
        )?;
        Ok(Some(format!("Task delegated to {link_name}: {task_summary}")))
    }

    /// Attempt agent federation for a complex task.
    /// Decomposes the task across specialized agent roles and returns a summary.
    fn try_federation(&self, goal_title: &str, summary: &str) -> Result<String> {
        let fed = crate::federation::AgentFederation::new(self.paths);
        let req = crate::federation::FederationRequest {
            task: goal_title.to_string(),
            max_agents: 4,
            context: summary.to_string(),
        };
        let result = fed.run(req)?;
        if result.success {
            self.emit(
                "agent:federated",
                &format!(
                    "federation {}: {} subtasks completed",
                    result.federation_id,
                    result.results.len()
                ),
            )?;
            crate::channels::notify_event(
                "federated",
                &format!(
                    "{}: {}",
                    result.federation_id,
                    result.final_output.chars().take(120).collect::<String>()
                ),
            );
            Ok(format!(
                "Federation {} completed: {} subtasks — {}",
                result.federation_id,
                result.results.len(),
                result.final_output.chars().take(200).collect::<String>()
            ))
        } else {
            log::warn!("federation {} failed — falling back to single-agent", result.federation_id);
            anyhow::bail!("federation run unsuccessful")
        }
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

        // (#3) Plugin block check — interceptors can block tools by pattern.
        // Checked before SecurityPolicy so plugins can deny even approved requests.
        let command = request.payload_json.as_deref().unwrap_or("");
        if let Some(reason) = self.plugins.borrow().should_block(command) {
            self.emit(
                "agent:tool_blocked_by_plugin",
                &format!("{} — {}", request.tool_name, reason),
            )?;
            state.last_outcome = Some("plugin_blocked".to_string());
            state.action_summary =
                Some(format!("Tool '{}' blocked by plugin policy: {reason}", request.tool_name));
            state.updated_at = self.clock.now_utc();
            return Ok(());
        }

        SecurityPolicy.validate_request(self.config, self.paths, &manifest, &request)?;
        let invocation_key = invocation_key(&manifest, &request);

        match LoopGuard.record(state, &invocation_key, DEFAULT_LOOP_GUARD_LIMIT) {
            GuardDecision::Allow => {}
            GuardDecision::Blocked { consecutive_count, pattern_len } => {
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

        // ── Shell Hook: tool.before ───────────────────────────────────────
        // Interceptor hooks can abort tool execution before it starts.
        let hooks = HookRunner::from_paths(self.paths);
        let tool_ctx = HookContext::new("tool.before", self.paths.data_dir.clone())
            .with_phase("act")
            .with_tool(&manifest.name, Some(request.id));

        if let Err(e) = hooks.fire_interceptor("tool.before", &tool_ctx, &manifest.name) {
            log::info!("hooks: tool.before interceptor blocked tool '{}': {e}", manifest.name);
            state.last_outcome = Some("blocked_by_hook".to_string());
            state.action_summary =
                Some(format!("Hook interceptor blocked tool '{}' execution: {e}", manifest.name));
            state.updated_at = self.clock.now_utc();
            return Ok(());
        }

        let execution =
            execute_request(self.paths, &manifest, &request, self.config.security.redact_secrets)?;
        self.store.mark_approval_consumed(request.id)?;
        sync_capabilities(self.tools, self.store, self.paths)?;
        self.emit("agent:tool_call", &format!("{} {}", manifest.name, request.summary))?;

        // ── Shell Hook: tool.after ────────────────────────────────────────
        let tool_after_ctx = HookContext::new("tool.after", self.paths.data_dir.clone())
            .with_phase("act")
            .with_tool(&manifest.name, Some(request.id));
        hooks.fire_observer("tool.after", &tool_after_ctx, &manifest.name);

        // (#5) Plugin output rewrite — transform tool output through plugin pipelines.
        // Each plugin's `tool_rewrite` commands receive the output on stdin
        // and return the rewritten output on stdout, chained in load order.
        let final_summary =
            self.plugins.borrow().rewrite_tool_output(&manifest.name, &execution.summary);

        state.last_outcome = Some("tool_executed".to_string());
        state.action_summary = Some(final_summary);
        state.updated_at = self.clock.now_utc();
        Ok(())
    }

    fn block_for_usage_budget(
        &self,
        state: &mut SessionState,
        mode: UsageBudgetMode,
    ) -> Result<bool> {
        let budgets = UsageBudgetPolicy::load_or_default(&self.paths.budgets_file)?;
        let decision = budgets.rule(mode).check_attempts(&state.provider_attempts, mode);
        if !decision.blocked {
            return Ok(false);
        }
        state.last_outcome = Some("budget_exhausted".to_string());
        state.action_summary = Some(decision.summary);
        state.updated_at = self.clock.now_utc();
        self.emit("agent:usage_budget_blocked", "Usage budget blocked another backend call.")?;
        Ok(true)
    }
}

/// Derive a confidence score from the reason code.
///
/// Explicit operator requests are near-certain; goal-driven decisions carry
/// meaningful uncertainty; budget/guard blocks are deterministic.
fn decision_confidence(reason_code: &str) -> f64 {
    match reason_code {
        "task_selected" => 0.95,
        "approved_tool_selected" => 0.99,
        "goal_selected" => 0.80,
        "waiting_on_dependencies" => 0.90,
        "stop_condition_met" => 1.0,
        "budget_exhausted" | "blocked_loop_guard" => 1.0,
        _ => 0.70,
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

/// Extract meaningful keywords from a goal title or task string to use as
/// speculative success criteria. Filters out short stop-words.
fn speculative_keywords(text: &str) -> Vec<String> {
    const MIN_LEN: usize = 4;
    const STOP: &[&str] = &[
        "with", "from", "that", "this", "have", "will", "been", "into", "when", "then", "also",
    ];
    text.split_whitespace()
        .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()).to_lowercase())
        .filter(|w| w.len() >= MIN_LEN && !STOP.contains(&w.as_str()))
        .collect()
}

/// Load the active hand (if any) and validate required tools are registered.
/// Missing required tools return an error; missing optional tools log a warning.
fn enforce_active_hand(paths: &PraxisPaths, tools: &impl ToolRegistry) -> Result<()> {
    let name = match std::fs::read_to_string(&paths.active_hand_file) {
        Ok(s) => {
            let trimmed = s.trim().to_string();
            if trimmed.is_empty() {
                return Ok(());
            }
            trimmed
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => {
            log::warn!("failed to read active hand file: {e}");
            return Ok(());
        }
    };

    let store = HandStore::load(&paths.hands_dir)?;
    let hand = store.get(&name).ok_or_else(|| {
        anyhow::anyhow!("active hand '{name}' not found in {}", paths.hands_dir.display())
    })?;

    let registered: std::collections::HashSet<String> =
        tools.list(paths)?.into_iter().map(|m| m.name).collect();

    let missing_required: Vec<&str> = hand
        .tools
        .required
        .iter()
        .filter(|t| !registered.contains(*t))
        .map(String::as_str)
        .collect();

    if !missing_required.is_empty() {
        anyhow::bail!(
            "active hand '{}' requires tools that are not registered: {}",
            hand.name,
            missing_required.join(", ")
        );
    }

    for tool in &hand.tools.optional {
        if !registered.contains(tool) {
            log::warn!("active hand '{}': optional tool '{tool}' is not registered", hand.name);
        }
    }

    log::info!(
        "active hand '{}' loaded: {} required tool(s), {} optional, {} skill(s)",
        hand.name,
        hand.tools.required.len(),
        hand.tools.optional.len(),
        hand.skills.load.len(),
    );

    Ok(())
}

/// Send a Telegram notification with Approve/Deny inline buttons for a pending approval request.
/// Best-effort — logs warnings on failure but does not propagate errors.
pub(crate) fn notify_approval_request(approval_id: i64, tool_name: &str, summary: &str) {
    use crate::messaging::TelegramBot;

    // ── Telegram ──────────────────────────────────────────────────────────
    let Ok(bot) = TelegramBot::from_env() else {
        return;
    };
    let Some(chat_id) = bot.primary_chat_id() else {
        return;
    };

    let text = format!(
        "⏳ Approval required — #{approval_id}\n\
         Tool: {tool_name}\n\
         {summary}\n\n\
         Or reply: /approve {approval_id} or /deny {approval_id}"
    );
    let approve_data = format!("approve:{approval_id}");
    let deny_data = format!("deny:{approval_id}");
    let buttons = vec![("✅ Approve", approve_data.as_str()), ("❌ Deny", deny_data.as_str())];
    if let Err(e) = bot.send_message_with_buttons(chat_id, &text, &buttons) {
        log::warn!("failed to send approval notification with buttons: {e}");
    }

    // ── Discord ───────────────────────────────────────────────────────────
    #[cfg(feature = "discord")]
    {
        use crate::messaging::DiscordClient;
        if let Ok(discord) = DiscordClient::from_env()
            && let Ok(channel_ids) = std::env::var("PRAXIS_DISCORD_CHANNEL_IDS")
        {
            // Send to the first configured Discord channel.
            if let Some(channel_id) = channel_ids.split(',').next().map(|s| s.trim().to_string()) {
                let discord_buttons =
                    vec![("✅ Approve", approve_data.as_str()), ("❌ Deny", deny_data.as_str())];
                if let Err(e) =
                    discord.send_message_with_buttons(&channel_id, &text, &discord_buttons)
                {
                    log::warn!("failed to send Discord approval notification with buttons: {e}");
                }
            }
        }
    }
}

pub(crate) fn handle_approval_callback(store: &dyn ApprovalStore, data: &str) -> Result<bool> {
    if let Some(id_str) = data.strip_prefix("approve:") {
        if let Ok(id) = id_str.parse::<i64>()
            && let Some(req) = store.get_approval(id)?
        {
            use crate::storage::ApprovalStatus;
            if req.status == ApprovalStatus::Pending {
                store.set_approval_status(
                    id,
                    ApprovalStatus::Approved,
                    Some("approved via Telegram button"),
                )?;
                log::info!("approval #{id} approved via Telegram button");
                return Ok(true);
            }
        }
    } else if let Some(id_str) = data.strip_prefix("deny:")
        && let Ok(id) = id_str.parse::<i64>()
        && let Some(req) = store.get_approval(id)?
    {
        use crate::storage::ApprovalStatus;
        if req.status == ApprovalStatus::Pending {
            store.set_approval_status(
                id,
                ApprovalStatus::Rejected,
                Some("denied via Telegram button"),
            )?;
            log::info!("approval #{id} denied via Telegram button");
            return Ok(true);
        }
    }
    Ok(false)
}
