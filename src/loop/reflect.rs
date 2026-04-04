use anyhow::{Context, Result};

use crate::{
    forensics::attach_session_id,
    identity::Goal,
    memory::{MemoryStore, NewHotMemory},
    quality::{EvalRunner, LocalEvalSuite, LocalReviewer, Reviewer, summarize},
    state::SessionState,
    storage::{
        ApprovalStore, EvalRunRecord, ProviderUsageStore, QualityStore, ReviewRecord, ReviewStatus,
        SessionQualityUpdate, SessionRecord, SessionStore,
    },
    tools::ToolRegistry,
};

use super::{AgentBackend, PraxisRuntime};

impl<'a, B, C, E, G, I, S, T> PraxisRuntime<'a, B, C, E, G, I, S, T>
where
    B: AgentBackend,
    C: crate::time::Clock,
    E: crate::events::EventSink,
    G: crate::identity::GoalParser,
    I: crate::identity::IdentityPolicy,
    S: SessionStore + MemoryStore + ApprovalStore + QualityStore + ProviderUsageStore,
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
        self.capture_session_memory(&stored, &final_outcome)?;
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
        })?;
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

pub(super) fn selected_goal(state: &SessionState) -> Option<Goal> {
    Some(Goal {
        id: state.selected_goal_id.clone()?,
        title: state.selected_goal_title.clone()?,
        completed: false,
        line_number: 0,
        blocked_by: Vec::new(),
        wake_when: None,
    })
}

fn final_outcome(initial: &str, review_status: ReviewStatus, eval_failures: usize) -> String {
    if review_status == ReviewStatus::Failed {
        "review_failed".to_string()
    } else if eval_failures > 0 {
        "eval_failed".to_string()
    } else {
        initial.to_string()
    }
}

fn compose_summary(
    action_summary: &str,
    review_summary: &str,
    eval_summary: crate::quality::EvalSummary,
    findings: &[String],
) -> String {
    let mut summary = format!(
        "{action_summary} {review_summary} Evals passed={}, failed={}, skipped={}.",
        eval_summary.passed, eval_summary.failed, eval_summary.skipped
    );

    if !findings.is_empty() {
        summary.push_str(" Findings: ");
        summary.push_str(&findings.join(" | "));
    }

    summary
}
