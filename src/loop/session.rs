use anyhow::Result;

use crate::{
    events::Event,
    state::{SessionPhase, SessionState},
    time::is_quiet_hours,
};

use super::{AgentBackend, PraxisRuntime, RunOptions, RunSummary};

impl<'a, B, C, E, G, I, S, T> PraxisRuntime<'a, B, C, E, G, I, S, T>
where
    B: AgentBackend,
    C: crate::time::Clock,
    E: crate::events::EventSink,
    G: crate::identity::GoalParser,
    I: crate::identity::IdentityPolicy,
    S: crate::storage::SessionStore
        + crate::storage::ApprovalStore
        + crate::storage::ProviderUsageStore
        + crate::storage::OperationalMemoryStore
        + crate::storage::AnatomyStore,
    T: crate::tools::ToolRegistry,
{
    pub(super) fn validate_options(&self, options: &RunOptions) -> Result<()> {
        if !options.once {
            anyhow::bail!("only `praxis run --once` is supported in the foundation milestone");
        }
        Ok(())
    }

    pub(super) fn should_defer_for_quiet_hours(
        &self,
        now: chrono::DateTime<chrono::Utc>,
        force: bool,
    ) -> Result<bool> {
        Ok(!force
            && is_quiet_hours(
                now,
                &self.config.instance.timezone,
                &self.config.runtime.quiet_hours_start,
                &self.config.runtime.quiet_hours_end,
            )?)
    }

    pub(super) fn defer_session(
        &self,
        now: chrono::DateTime<chrono::Utc>,
        task: Option<String>,
    ) -> Result<RunSummary> {
        let mut deferred_state = SessionState::new(now, task);
        deferred_state.action_summary =
            Some("Quiet hours active; session deferred until the next wake window.".to_string());
        deferred_state.finish("deferred_quiet_hours", now);
        deferred_state.save(&self.paths.state_file)?;
        self.emit(
            "agent:sleep_deferred",
            deferred_state
                .action_summary
                .as_deref()
                .unwrap_or("quiet hours"),
        )?;

        Ok(RunSummary {
            outcome: "deferred_quiet_hours".to_string(),
            phase: SessionPhase::Sleep,
            resumed: false,
            selected_goal_id: None,
            selected_goal_title: None,
            selected_task: deferred_state.requested_task.clone(),
            action_summary: deferred_state.action_summary.unwrap_or_default(),
        })
    }

    pub(super) fn load_or_create_state(
        &self,
        now: chrono::DateTime<chrono::Utc>,
        task: Option<String>,
    ) -> Result<SessionState> {
        Ok(match SessionState::load(&self.paths.state_file)? {
            Some(existing) if existing.is_incomplete() => {
                let mut existing = existing;
                if let Some(task) = task {
                    existing.requested_task = Some(task);
                }
                existing.resume_count += 1;
                existing.updated_at = now;
                existing
            }
            _ => SessionState::new(now, task),
        })
    }

    pub(super) fn emit(&self, kind: &str, detail: &str) -> Result<()> {
        self.events.emit(&Event {
            kind: kind.to_string(),
            detail: detail.to_string(),
        })
    }
}
