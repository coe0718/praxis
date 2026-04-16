use anyhow::{Context, Result};

use crate::{
    anomaly::{SystemSnapshot, record_snapshot as record_system_snapshot},
    evolution::{
        ChangeKind, EvolutionProposal, EvolutionStore, ProposalStatus, render_self_evolution_doc,
    },
    examples::{SyntheticExample, build_context, examples_file, is_useful_outcome, record_example},
    forensics::attach_session_id,
    identity::Goal,
    memory::{MemoryLinkStore, MemoryStore, NewDoNotRepeat, NewHotMemory, NewKnownBug},
    paths::PraxisPaths,
    postmortem::append_postmortem,
    quality::{EvalRunner, LocalEvalSuite, LocalReviewer, Reviewer, summarize},
    score::{ScoreWeights, SessionScore, SessionScoreInput, record_score},
    skills::{SkillSynthesizer, synthesis::SkillSynthesisInput},
    state::SessionState,
    storage::{
        AnatomyStore, ApprovalStore, DecisionReceiptStore, EvalRunRecord, OperationalMemoryStore,
        ProviderUsageStore, QualityStore, ReviewRecord, ReviewStatus, SessionQualityUpdate,
        SessionRecord, SessionStore,
    },
    tools::ToolRegistry,
};

use super::{
    AgentBackend, PraxisRuntime,
    outcome::{compose_summary, final_outcome},
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
    pub(super) fn reflect(&self, state: &mut SessionState) -> Result<()> {
        let ended_at = self.clock.now_utc();
        let initial_outcome = state
            .last_outcome
            .clone()
            .unwrap_or_else(|| "idle".to_string());
        let record = SessionRecord {
            day: self.identity.read_day_count(self.paths)?,
            started_at: state.started_at,
            ended_at,
            outcome: initial_outcome.clone(),
            selected_goal_id: state.selected_goal_id.clone(),
            selected_goal_title: state.selected_goal_title.clone(),
            selected_task: state.selected_task_label(),
            action_summary: state.action_summary.clone().unwrap_or_default(),
            phase_durations_json: serde_json::json!({
                "orient": 0,
                "decide": 0,
                "act": 0,
                "reflect": 0
            })
            .to_string(),
            repeated_reads_avoided: state.repeated_reads_avoided as i64,
        };
        let mut stored = self.store.record_session(&record)?;
        attach_session_id(&self.paths.database_file, state.started_at, stored.id)?;
        self.store
            .record_provider_attempts(stored.id, &state.provider_attempts)?;

        let review = LocalReviewer.review(self.paths, stored.selected_goal_id.as_deref())?;
        self.store.record_review(&ReviewRecord {
            session_id: stored.id,
            goal_id: stored.selected_goal_id.clone(),
            status: review.status,
            summary: review.summary.clone(),
            findings_json: serde_json::to_string(&review.findings)
                .context("failed to serialize reviewer findings")?,
            reviewed_at: ended_at,
        })?;

        let eval_results = LocalEvalSuite.run(self.paths)?;
        for result in &eval_results {
            self.store.record_eval_run(&EvalRunRecord {
                session_id: stored.id,
                eval_id: result.eval_id.clone(),
                eval_name: result.name.clone(),
                status: result.status,
                severity: result.severity,
                summary: result.summary.clone(),
                evaluated_at: ended_at,
            })?;
        }

        let eval_summary = summarize(&eval_results);
        let final_outcome = final_outcome(&initial_outcome, review.status, eval_summary.failed);
        let final_summary = compose_summary(
            &stored.action_summary,
            &review.summary,
            eval_summary,
            &review.findings,
        );

        self.store.update_session_quality(
            stored.id,
            &SessionQualityUpdate {
                outcome: final_outcome.clone(),
                action_summary: final_summary.clone(),
                reviewer_passes: i64::from(review.status == ReviewStatus::Passed),
                reviewer_failures: i64::from(review.status == ReviewStatus::Failed),
                eval_passes: eval_summary.passed as i64,
                eval_failures: eval_summary.failed as i64,
            },
        )?;

        stored.outcome = final_outcome.clone();
        stored.action_summary = final_summary.clone();

        let score_input =
            build_score_input(state, &stored.outcome, stored.selected_goal_id.is_some());
        let score = SessionScore::compute(&score_input, &ScoreWeights::default())
            .with_session_id(stored.id);
        if let Err(e) = record_score(&self.paths.score_file, &score) {
            log::warn!("failed to record session score: {e}");
        }

        if is_useful_outcome(&stored.outcome) {
            let ctx = build_context(
                stored.selected_goal_title.as_deref(),
                &stored.action_summary,
                0,
                state.tool_invocation_hashes.len(),
            );
            let mut example = SyntheticExample::new(ctx, &stored.action_summary, &stored.outcome)
                .with_session_id(stored.id);
            if let Some(ref gid) = stored.selected_goal_id {
                example = example.with_goal_id(gid);
            }
            example = example.with_quality_score(score.composite);
            if let Err(e) = record_example(&examples_file(self.paths), &example) {
                log::warn!("failed to record synthetic example: {e}");
            }
        }

        let snapshot = SystemSnapshot::capture(&self.paths.data_dir, Some(stored.outcome.clone()));
        if let Err(e) = record_system_snapshot(&self.paths.system_anomalies_file, &snapshot) {
            log::warn!("failed to record system snapshot: {e}");
        }

        if let Err(e) = maybe_propose_evolution(self.paths, &stored.outcome, &score, stored.id) {
            log::warn!("evolution proposal failed: {e}");
        }

        append_postmortem(
            self.paths,
            &stored,
            &final_outcome,
            &review.summary,
            &review.findings,
            eval_summary,
            &eval_results,
        )?;
        self.capture_session_memory(&stored, &final_outcome)?;
        self.capture_operational_memory(
            stored.id,
            state,
            review.status,
            &review.summary,
            &review.findings,
            eval_summary.failed,
        )?;
        crate::context::record_context_feedback(
            &self.paths.context_adaptation_file,
            ended_at,
            &state.context_sources,
            &final_outcome,
        )?;
        self.maybe_synthesize_skill(state, &final_outcome, &stored.action_summary, ended_at)?;
        self.identity.append_journal(self.paths, &stored)?;
        self.identity.append_metrics(self.paths, &stored)?;
        self.emit_review_events(review.status, eval_summary.failed)?;

        state.action_summary = Some(final_summary);
        state.finish(final_outcome, ended_at);
        state.updated_at = ended_at;
        Ok(())
    }

    fn capture_session_memory(
        &self,
        stored: &crate::storage::StoredSession,
        outcome: &str,
    ) -> Result<()> {
        let memory_summary = format!(
            "Session outcome {} with summary: {}",
            outcome, stored.action_summary
        );
        self.store.insert_hot_memory(NewHotMemory {
            content: memory_summary,
            summary: Some(outcome.to_string()),
            importance: 0.7,
            tags: vec!["session".to_string(), "foundation".to_string()],
            expires_at: None,
            memory_type: Default::default(),
        })?;
        Ok(())
    }

    fn capture_operational_memory(
        &self,
        session_id: i64,
        state: &SessionState,
        review_status: ReviewStatus,
        review_summary: &str,
        findings: &[String],
        eval_failures: usize,
    ) -> Result<()> {
        if review_status != ReviewStatus::Failed && eval_failures == 0 {
            return Ok(());
        }

        let target = state.selected_task_label().unwrap_or_else(|| {
            state
                .selected_goal_id
                .clone()
                .zip(state.selected_goal_title.clone())
                .map(|(id, title)| format!("{id}: {title}"))
                .unwrap_or_else(|| "this workflow".to_string())
        });
        let severity = if review_status == ReviewStatus::Failed {
            "review_failed"
        } else {
            "eval_failed"
        };

        self.store.record_do_not_repeat(NewDoNotRepeat {
            statement: format!(
                "Do not treat {} as complete until reviewer and eval checks pass cleanly.",
                target
            ),
            tags: vec![severity.to_string(), "operations".to_string()],
            severity: severity.to_string(),
            source_session_id: Some(session_id),
            expires_at: None,
        })?;
        self.store.record_known_bug(NewKnownBug {
            signature: target,
            symptoms: review_summary.to_string(),
            fix_summary: findings.first().cloned().unwrap_or_else(|| {
                "Repair the failing work, rerun verification, and only then mark the session complete."
                    .to_string()
            }),
            tags: vec![severity.to_string(), "quality".to_string()],
            source_session_id: Some(session_id),
        })?;
        Ok(())
    }

    fn maybe_synthesize_skill(
        &self,
        state: &SessionState,
        outcome: &str,
        action_summary: &str,
        ended_at: chrono::DateTime<chrono::Utc>,
    ) -> Result<()> {
        let input = SkillSynthesisInput {
            outcome,
            goal_title: state.selected_goal_title.as_deref(),
            tool_call_count: state.tool_invocation_hashes.len(),
            action_summary,
            session_ended_at: ended_at,
        };
        SkillSynthesizer.maybe_draft(&self.paths.skills_drafts_dir, &input)?;
        Ok(())
    }

    fn emit_review_events(&self, review_status: ReviewStatus, eval_failures: usize) -> Result<()> {
        let review_kind = match review_status {
            ReviewStatus::Passed => "agent:review_passed",
            ReviewStatus::Failed => "agent:review_failed",
            ReviewStatus::Skipped => "agent:review_skipped",
        };
        self.emit(review_kind, "Reflect completed reviewer checks.")?;

        let eval_kind = if eval_failures > 0 {
            "agent:eval_failed"
        } else {
            "agent:eval_passed"
        };
        self.emit(eval_kind, "Reflect completed operator eval checks.")?;
        self.emit(
            "agent:session_complete",
            "Reflect finalized the session outcome.",
        )
    }
}

fn maybe_propose_evolution(
    paths: &PraxisPaths,
    outcome: &str,
    score: &SessionScore,
    session_id: i64,
) -> Result<()> {
    if matches!(
        outcome,
        "idle" | "skipped" | "deferred" | "approved_tool_selected"
    ) {
        return Ok(());
    }

    let store = EvolutionStore::from_paths(paths);
    let pending = store.with_status(ProposalStatus::Proposed)?;

    // Cap auto-proposals to avoid noise.
    if pending.len() >= 3 {
        return Ok(());
    }

    let proposal = if outcome == "review_failed" {
        let title = "Review quality gate criteria after session review failure";
        if pending
            .iter()
            .any(|p| p.title.starts_with("Review quality gate"))
        {
            return Ok(());
        }
        EvolutionProposal::new(
            title,
            format!(
                "Session {session_id} ended with review_failed. The reviewer may be catching a \
                 fixable pattern. Check AGENTS.md and the reviewer findings from this session."
            ),
            ChangeKind::Config,
            "Review `[quality]` thresholds in praxis.toml or update AGENTS.md guidance to \
             address the recurring failure pattern.",
        )
        .with_evidence(vec![session_id])
    } else if outcome == "eval_failed" {
        let title = "Address recurring eval failures";
        if pending
            .iter()
            .any(|p| p.title.starts_with("Address recurring eval"))
        {
            return Ok(());
        }
        EvolutionProposal::new(
            title,
            format!(
                "Session {session_id} ended with eval_failed. Review the `evals/` directory \
                 for false positives or fix the underlying defect."
            ),
            ChangeKind::Config,
            "Update or remove evals that consistently false-positive, or resolve the root \
             cause they are detecting.",
        )
        .with_evidence(vec![session_id])
    } else if score.composite < 0.5 && score.follow_through < 0.5 {
        let title = "Improve goal follow-through rate";
        if pending
            .iter()
            .any(|p| p.title.starts_with("Improve goal follow-through"))
        {
            return Ok(());
        }
        EvolutionProposal::new(
            title,
            format!(
                "Session {session_id} scored {:.2} composite with {:.2} follow-through. \
                 Goals are being selected but not completed.",
                score.composite, score.follow_through
            ),
            ChangeKind::Identity,
            "Consider: (1) breaking large goals into smaller milestones in GOALS.md, \
             (2) increasing context budget in praxis.toml, \
             (3) reviewing PATTERNS.md for recurring obstacles.",
        )
        .with_evidence(vec![session_id])
    } else {
        return Ok(());
    };

    store.propose(&proposal)?;
    render_self_evolution_doc(paths)?;
    log::info!("evolution: proposed '{}'", proposal.title);
    Ok(())
}

fn build_score_input(
    state: &SessionState,
    outcome: &str,
    goal_was_selected: bool,
) -> SessionScoreInput {
    let proactive = state.requested_task.is_none();
    let goal_completed = goal_was_selected
        && outcome != "review_failed"
        && outcome != "eval_failed"
        && outcome != "idle";
    let tool_calls = state.tool_invocation_hashes.len() as u32;
    SessionScoreInput {
        proactive_wake_hits: u32::from(proactive && goal_completed),
        proactive_wakes_total: u32::from(proactive),
        goal_completed,
        goal_was_selected,
        approvals_passed: tool_calls,
        approvals_total: tool_calls,
        operator_intervened: state.resume_count > 0,
    }
}

pub(super) fn selected_goal(state: &SessionState) -> Option<Goal> {
    Some(Goal {
        id: state.selected_goal_id.clone()?,
        title: state.selected_goal_title.clone()?,
        completed: false,
        line_number: 0,
        parent_id: None,
        blocked_by: Vec::new(),
        wake_when: None,
    })
}
