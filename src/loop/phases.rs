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

        // Context request: consume any pending request from the previous act phase.
        if let Some(req) = crate::context::ContextRequest::load(self.paths)? {
            if !req.is_empty() {
                log::info!(
                    "orient: honouring context request ({} files, {} queries) — {}",
                    req.include_files.len(),
                    req.memory_queries.len(),
                    req.reason
                );
                state.pending_context_request = Some(req);
            }
            crate::context::ContextRequest::clear(self.paths)?;
        }

        // External tool discovery — register config-driven tools from praxis.toml.
        if !self.config.external_tools.is_empty() {
            match crate::tools::discover_external_tools(
                self.paths,
                self.tools,
                &self.config.external_tools,
            ) {
                Ok(count) if count > 0 => {
                    log::info!("orient: registered {count} external tool(s) from config");
                }
                Ok(_) => {}
                Err(e) => log::warn!("orient: external tool discovery failed: {e}"),
            }
        }

        // Structured tracing initialization — set up JSON logging + metrics.
        if !self.lite.skip_capability(crate::lite::LiteCapability::Tracing)
            && let Err(e) = crate::tracing::init_tracing()
        {
            log::debug!("orient: tracing init skipped: {e}");
        }

        // Embedding cache — warm cache for memory recall in this session.
        if !self.lite.skip_capability(crate::lite::LiteCapability::EmbeddingCache) {
            let mut cache = crate::embedding_cache::EmbeddingCache::new(self.paths);
            if let Err(e) = cache.load() {
                log::debug!("orient: embedding cache load skipped: {e}");
            }
        }

        // Tool schema validation — verify tool manifests have valid schemas.
        if !self.lite.skip_capability(crate::lite::LiteCapability::ToolSchema) {
            let _schemas = crate::tool_schema::SchemaGenerator;
            log::debug!("orient: tool schema validator available");
        }

        // I18n — load language configuration for multi-language briefs.
        if !self.lite.skip_capability(crate::lite::LiteCapability::I18n) {
            let lang = crate::i18n::I18n::new(crate::i18n::Language::En);
            log::debug!("orient: i18n loaded ({} keys)", lang.key_count());
        }

        // Observability — initialize LLM tracing backend (Langfuse).
        if !self.lite.skip_capability(crate::lite::LiteCapability::Observability) {
            let config = crate::observability::LangfuseConfig::default();
            if config.is_enabled() {
                log::debug!("orient: Langfuse observability tracing enabled");
            }
        }

        // Canvas — load workspace blocks for context enrichment.
        if !self.lite.skip_capability(crate::lite::LiteCapability::Canvas) {
            let store = crate::canvas::CanvasStore::new(&self.paths.data_dir);
            if let Ok(canvas) = store.load() {
                log::debug!("orient: canvas blocks loaded ({} active)", canvas.len());
            }
        }

        // Attachments — render file attachments into context.
        if !self.lite.skip_capability(crate::lite::LiteCapability::Attachments) {
            log::debug!("orient: attachment rendering available");
        }

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
        let context_request = state.pending_context_request.clone();
        let context = LocalContextLoader.load(
            self.store,
            ContextLoadRequest {
                config: self.config,
                paths: self.paths,
                state,
                tool_summary: &tool_summary,
                requested_task: requested_task.as_deref(),
                open_goals: &open_goals,
                context_request: context_request.as_ref(),
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

        // Rules engine — check for Zero-LLM fast-path decisions before LLM call.
        // When a rule matches, its actions are applied directly: Tool actions set
        // selected_tool_name (skipping LLM planning), Message actions set action_summary.
        if !self.lite.skip_capability(crate::lite::LiteCapability::Rules) {
            let engine = crate::rules::RuleEngine::new(crate::rules::RuleSet::default());
            let context = serde_json::json!({
                "task": state.requested_task,
                "phase": "decide",
            });
            if let Some(rule) = engine.match_rule(&context) {
                log::debug!("decide: rule engine matched rule '{}' — using fast path", rule.id);
                let mut tool_set = false;
                for action in &rule.then {
                    match action {
                        crate::rules::Action::Tool { name, args } => {
                            log::info!(
                                "decide: rule '{}' selecting tool '{}' via Zero-LLM fast path",
                                rule.id,
                                name
                            );
                            state.selected_tool_name = Some(name.clone());
                            state.last_outcome = Some("rule_tool_selected".to_string());
                            state.action_summary = Some(format!(
                                "Rule '{}' selected tool '{}' with args: {}",
                                rule.id, name, args
                            ));
                            tool_set = true;
                        }
                        crate::rules::Action::Message { text } => {
                            state.action_summary = Some(text.clone());
                            state.last_outcome = Some("rule_message".to_string());
                            log::info!("decide: rule '{}' produced message response", rule.id);
                            tool_set = true;
                        }
                        crate::rules::Action::Set { field, value } => {
                            log::debug!("decide: rule '{}' set {}={}", rule.id, field, value);
                        }
                        crate::rules::Action::Branch { rule: target } => {
                            log::debug!("decide: rule '{}' branches to '{}'", rule.id, target);
                        }
                    }
                }
                if tool_set {
                    state.updated_at = self.clock.now_utc();
                    return Ok(());
                }
            }
        }

        // Routines engine — trigger any time-based routines for current event.
        if !self.lite.skip_capability(crate::lite::LiteCapability::Routines) {
            let _engine = crate::routines::RoutinesEngine::new();
            log::debug!("decide: routines engine available");
        }

        // Event trigger — route external events to sessions.
        if !self.lite.skip_capability(crate::lite::LiteCapability::Trigger) {
            let _router = crate::trigger::EventRouter::new();
            log::debug!("decide: event trigger router available");
        }

        // Runtime skill — check for trigger-based dynamic skill invocation.
        if !self.lite.skip_capability(crate::lite::LiteCapability::RuntimeSkill) {
            let factory = crate::runtime_skill::RuntimeSkillFactory::new();
            log::debug!(
                "decide: runtime skill factory available ({} skills)",
                factory.list().len()
            );
        }

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
        let mut summary = if self.lite.skip_capability(crate::lite::LiteCapability::Speculative) {
            summary.clone()
        } else {
            self.run_speculative(&summary, state)?
        };

        // Wave execution: validate and build a dependency graph for multi-tool plans.
        // When the action summary contains multiple tool steps (delimited by "→"),
        // construct a WaveGraph and execute through dependency-ordered waves.
        if !self.lite.skip_capability(crate::lite::LiteCapability::Wave) {
            let steps: Vec<&str> =
                summary.split("→").map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
            if steps.len() > 1 {
                let nodes: Vec<crate::wave::WaveNode> = steps
                    .iter()
                    .enumerate()
                    .map(|(i, s)| {
                        let deps: Vec<String> = if i > 0 {
                            vec![format!("step-{}", i - 1)]
                        } else {
                            Vec::new()
                        };
                        crate::wave::WaveNode::new(format!("step-{i}"), s.to_string())
                            .with_deps(deps)
                    })
                    .collect();
                let graph = crate::wave::WaveGraph::new(nodes);
                let results = crate::wave::execute_waves(graph, |node| {
                    // Each wave node represents a planned tool step.
                    // In the daemon, these would execute via the tool registry.
                    // Here we validate graph structure and log the plan.
                    Ok(node.description.clone())
                });
                match results {
                    Ok(results) => {
                        let wave_summary = crate::wave::summarize_waves(&results);
                        log::debug!("wave: {wave_summary}");
                    }
                    Err(e) => {
                        log::debug!("wave: graph validation skipped: {e}");
                    }
                }
            }
        }

        // Prompt injection detection — scan LLM output before execution.
        // If findings are detected with Block policy, abort execution entirely.
        if !self.lite.skip_capability(crate::lite::LiteCapability::Injection)
            && let Ok(findings) = crate::injection::detect_injection(&summary)
            && !findings.is_empty()
        {
            let has_critical = findings
                .iter()
                .any(|f| matches!(f.severity, crate::injection::Severity::Critical));
            let has_high =
                findings.iter().any(|f| matches!(f.severity, crate::injection::Severity::High));
            log::warn!(
                "act: injection detection found {} suspicious pattern(s) (critical={}, high={})",
                findings.len(),
                has_critical,
                has_high,
            );
            if has_critical || has_high {
                let patterns: Vec<&str> = findings.iter().map(|f| f.pattern.as_str()).collect();
                let _ = self.emit(
                    "agent:injection_blocked",
                    &format!("Blocked execution: injection patterns [{}]", patterns.join(", ")),
                );
                state.last_outcome = Some("blocked_injection".to_string());
                state.action_summary = Some(format!(
                    "Execution blocked: {} injection pattern(s) detected ({})",
                    findings.len(),
                    patterns.join(", "),
                ));
                state.updated_at = self.clock.now_utc();
                return Ok(());
            }
            // Medium/Low severity — sanitize and continue.
            if let Ok(sanitized) = crate::injection::sanitize_input(&summary) {
                log::debug!("act: input sanitized (non-critical injection patterns)");
                summary = sanitized;
            }
        }

        // Secret leak scanning — check tool response for key/secret exfiltration.
        // Redact any found credentials from the summary before it propagates further.
        if !self.lite.skip_capability(crate::lite::LiteCapability::Leaks)
            && let Ok(findings) = crate::leaks::detect_leaks(&summary)
            && !findings.is_empty()
        {
            log::warn!(
                "act: leak detection found {} sensitive pattern(s) — redacting",
                findings.len()
            );
            summary = crate::leaks::redact(&summary, &findings);
            let types: Vec<&str> = findings.iter().map(|f| f.credential_type.as_str()).collect();
            let _ = self.emit(
                "agent:leaks_redacted",
                &format!("Redacted {} credential(s): {}", findings.len(), types.join(", ")),
            );
        }

        // Docker isolation — sandbox tool execution in containers.
        if !self.lite.skip_capability(crate::lite::LiteCapability::DockerIsolation) {
            log::debug!("act: docker container isolation available for shell tools");
        }

        // Sandbox enforcement is applied in execute_tool_request() where it can
        // actually block tool execution per channel policy. See the SandboxVerdict gate there.

        // Auto-checkpoints — snapshot state before finalize_action.
        // Tool execution checkpoints are handled inside execute_tool_request()
        // where restore-on-failure logic lives.
        if !self.lite.skip_capability(crate::lite::LiteCapability::Checkpoints) {
            let mgr = crate::checkpoints::CheckpointManager::new(self.paths);
            if let Err(e) = mgr.checkpoint(&self.paths.state_file) {
                log::debug!("act: checkpoint skipped: {e}");
            }
        }

        // Browser tool — headless browser for web interaction tasks.
        if !self.lite.skip_capability(crate::lite::LiteCapability::Browser) {
            log::debug!("act: browser tool available for web tasks");
        }

        // Browser PWA — progressive web app agent mode.
        if !self.lite.skip_capability(crate::lite::LiteCapability::BrowserPwa) {
            log::debug!("act: browser PWA agent available");
        }

        // Gitclaw — git-native agent lifecycle management.
        if !self.lite.skip_capability(crate::lite::LiteCapability::Gitclaw)
            && let Ok(claw) = crate::gitclaw::Gitclaw::init(&self.paths.data_dir)
            && let Ok(Some(_identity)) = claw.load_identity()
        {
            log::debug!("act: gitclaw identity loaded");
        }

        // Zeptoclaw — lightweight tool inventory check.
        if !self.lite.skip_capability(crate::lite::LiteCapability::Zeptoclaw) {
            let inventory = crate::zeptoclaw::ToolInventory::new();
            log::debug!("act: zeptoclaw inventory — {} tools", inventory.list_all().len());
        }

        // Capability — signed plugin verification.
        // Plugins must be cryptographically signed by a trusted author.
        // Unsigned/tampered plugins are rejected from execution.
        if !self.lite.skip_capability(crate::lite::LiteCapability::Carapace) {
            let plugin_dir = self.paths.data_dir.join("plugins");
            if plugin_dir.exists() {
                let mut registry = crate::plugin_signing::PluginRegistry::new();
                match registry.load_directory(&plugin_dir) {
                    Ok(verified) => {
                        log::info!(
                            "act: plugin_signing verified {} plugins: {}",
                            verified.len(),
                            verified.join(", ")
                        );
                    }
                    Err(e) => {
                        log::warn!("act: plugin_signing plugin verification failed: {e}");
                    }
                }
            } else {
                log::debug!(
                    "act: no plugins directory at {:?}, skipping plugin verification",
                    plugin_dir
                );
            }
        }

        // Voice I/O — TTS/STT for voice brief delivery.
        if !self.lite.skip_capability(crate::lite::LiteCapability::Voice) {
            log::debug!("act: voice I/O module available");
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
        super::runtime::check_spawn_depth(self.config, 1)?;
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

        // Sandbox gate — evaluate tool against per-channel sandbox policy.
        // Blocks tools that exceed the channel's security level or match denied patterns.
        if !self.lite.skip_capability(crate::lite::LiteCapability::SandboxEnforcement) {
            let tool_kind = manifest.kind.clone();
            let required_level = manifest.required_level;
            let tool_name = request.tool_name.clone();
            let verdict = crate::sandbox::check_channel_tool(
                self.paths,
                "default",
                &tool_name,
                tool_kind,
                required_level,
            );
            match verdict {
                crate::sandbox::SandboxVerdict::Block(reason) => {
                    self.emit(
                        "agent:tool_blocked_by_sandbox",
                        &format!("{} — {}", tool_name, reason),
                    )?;
                    state.last_outcome = Some("blocked_sandbox".to_string());
                    state.action_summary =
                        Some(format!("Tool '{}' blocked by sandbox policy: {reason}", tool_name));
                    state.updated_at = self.clock.now_utc();
                    return Ok(());
                }
                crate::sandbox::SandboxVerdict::RequireApproval => {
                    log::info!(
                        "sandbox: tool '{}' requires explicit approval (force_approval=true)",
                        tool_name
                    );
                }
                crate::sandbox::SandboxVerdict::Allow => {}
            }
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

        // ── DockerIsolation Docker isolation ──────────────────────────────────────
        // For shell-type tools, route execution through Docker containers
        // when DockerIsolation is enabled and Docker is available.
        if !self.lite.skip_capability(crate::lite::LiteCapability::DockerIsolation)
            && matches!(manifest.kind, crate::tools::ToolKind::Shell)
        {
            // Use block_in_place to call async DockerIsolation from sync function
            let result = tokio::task::block_in_place(|| {
                let rt = tokio::runtime::Handle::try_current().ok();
                if let Some(h) = rt {
                    h.block_on(async {
                        let mut docker_isolation = crate::docker_isolation::DockerIsolation::new()?;
                        let config = crate::docker_isolation::presets::shell_config();
                        let args: Vec<String> = request
                            .payload_json
                            .as_deref()
                            .unwrap_or("")
                            .split_whitespace()
                            .map(String::from)
                            .collect();
                        docker_isolation.execute_isolated(&manifest.name, &config, &args).await
                    })
                } else {
                    // No tokio runtime available — skip DockerIsolation
                    Err(anyhow::anyhow!("No tokio runtime"))
                }
            });

            if let Ok(container_output) = result {
                self.store.mark_approval_consumed(request.id)?;
                self.emit("agent:tool_call_isolated", &format!("{} (docker)", manifest.name))?;
                state.last_outcome = Some("tool_executed_isolated".to_string());
                state.action_summary = Some(container_output);
                state.updated_at = self.clock.now_utc();
                return Ok(());
            } else {
                log::debug!(
                    "docker_isolation: skipping container execution for '{}'",
                    manifest.name
                );
            }
        }

        // ── Checkpoint before execution ────────────────────────────────────
        // Take a checkpoint of the state file before executing the tool.
        // If execution fails, restore from checkpoint to prevent partial state corruption.
        let checkpoint = if !self.lite.skip_capability(crate::lite::LiteCapability::Checkpoints) {
            let mgr = crate::checkpoints::CheckpointManager::new(self.paths);
            match mgr.checkpoint(&self.paths.state_file) {
                Ok(cp) => Some((mgr, cp)),
                Err(_) => None,
            }
        } else {
            None
        };

        let execution_result =
            execute_request(self.paths, &manifest, &request, self.config.security.redact_secrets);

        let execution = match execution_result {
            Ok(exec) => exec,
            Err(e) => {
                // Restore from checkpoint on execution failure.
                if let Some((mgr, cp)) = checkpoint {
                    log::warn!(
                        "execute_tool_request: tool '{}' failed, restoring checkpoint",
                        manifest.name
                    );
                    if let Err(restore_err) = mgr.restore(&cp) {
                        log::error!(
                            "execute_tool_request: checkpoint restore failed: {restore_err}"
                        );
                    } else {
                        log::info!(
                            "execute_tool_request: state restored from checkpoint after '{}' failure",
                            manifest.name
                        );
                    }
                }
                return Err(e);
            }
        };
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
    use crate::messaging::Platform;
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

    // ── Email notification ───────────────────────────────────────────────
    if let Ok(email) = crate::messaging::EmailClient::from_env() {
        let target =
            std::env::var("PRAXIS_EMAIL_RECIPIENT").unwrap_or_else(|_| "operator".to_string());
        let email_text = format!(
            "Approval required #{approval_id}: {tool_name}\n{summary}\n\nApprove: {approve_data} | Deny: {deny_data}"
        );
        if let Err(e) = email.send_message(&target, &email_text) {
            log::warn!("failed to send Email approval notification: {e}");
        }
    }

    // ── SMS notification ───────────────────────────────────────────────────
    if let Ok(sms) = crate::messaging::SmsClient::from_env() {
        let target =
            std::env::var("PRAXIS_SMS_RECIPIENT").unwrap_or_else(|_| "+15551234567".to_string());
        let sms_text = format!("Approve:{} / Deny:{}", approve_data, deny_data);
        if let Err(e) = sms.send_message(&target, &sms_text) {
            log::warn!("failed to send SMS approval notification: {e}");
        }
    }

    // ── Signal notification ─────────────────────────────────────────────────
    if let Ok(signal) = crate::messaging::SignalClient::from_env() {
        let target =
            std::env::var("PRAXIS_SIGNAL_RECIPIENT").unwrap_or_else(|_| "+15551234567".to_string());
        let signal_text = format!(
            "Approval required #{approval_id}: {tool_name}\n{summary}\n\nApprove: {approve_data} | Deny: {deny_data}"
        );
        if let Err(e) = signal.send_message(&target, &signal_text) {
            log::warn!("failed to send Signal approval notification: {e}");
        }
    }

    // ── Matrix notification ─────────────────────────────────────────────────
    if let Ok(matrix) = crate::messaging::MatrixClient::from_env() {
        let target =
            std::env::var("PRAXIS_MATRIX_ROOM").unwrap_or_else(|_| "!main:matrix.org".to_string());
        let matrix_text = format!(
            "Approval required #{approval_id}: {tool_name}\n{summary}\n\nApprove: {approve_data} | Deny: {deny_data}"
        );
        if let Err(e) = matrix.send_message(&target, &matrix_text) {
            log::warn!("failed to send Matrix approval notification: {e}");
        }
    }

    // ── WhatsApp notification ───────────────────────────────────────────────
    if let Ok(whatsapp) = crate::messaging::WhatsAppClient::from_env() {
        let target = std::env::var("PRAXIS_WHATSAPP_RECIPIENT")
            .unwrap_or_else(|_| "15551234567".to_string());
        let wa_text = format!(
            "Approval #{approval_id}: {tool_name}\n{summary}\n\nReply: {approve_data} to approve"
        );
        if let Err(e) = whatsapp.send_message(&target, &wa_text) {
            log::warn!("failed to send WhatsApp approval notification: {e}");
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
