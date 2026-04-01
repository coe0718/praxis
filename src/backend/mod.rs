mod claude;

use std::env;

use anyhow::{Context, Result, bail};

use crate::{config::AppConfig, identity::Goal};

pub use claude::ClaudeBackend;

pub trait AgentBackend {
    fn name(&self) -> &'static str;
    fn plan_action(&self, goal: Option<&Goal>, task: Option<&str>) -> Result<String>;
    fn finalize_action(
        &self,
        planned_summary: &str,
        goal: Option<&Goal>,
        task: Option<&str>,
    ) -> Result<String>;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct StubBackend;

pub enum ConfiguredBackend {
    Stub(StubBackend),
    Claude(ClaudeBackend),
}

impl ConfiguredBackend {
    pub fn from_config(config: &AppConfig) -> Result<Self> {
        match config.agent.backend.as_str() {
            "stub" => Ok(Self::Stub(StubBackend)),
            "claude" => Ok(Self::Claude(ClaudeBackend::from_config(config)?)),
            other => bail!("unsupported backend {other}"),
        }
    }

    pub fn validate_environment(config: &AppConfig) -> Result<()> {
        if config.agent.backend == "claude" {
            let _ = env::var("ANTHROPIC_API_KEY")
                .context("ANTHROPIC_API_KEY is required when agent.backend = \"claude\"")?;
        }
        Ok(())
    }
}

impl AgentBackend for ConfiguredBackend {
    fn name(&self) -> &'static str {
        match self {
            Self::Stub(inner) => inner.name(),
            Self::Claude(inner) => inner.name(),
        }
    }

    fn plan_action(&self, goal: Option<&Goal>, task: Option<&str>) -> Result<String> {
        match self {
            Self::Stub(inner) => inner.plan_action(goal, task),
            Self::Claude(inner) => inner.plan_action(goal, task),
        }
    }

    fn finalize_action(
        &self,
        planned_summary: &str,
        goal: Option<&Goal>,
        task: Option<&str>,
    ) -> Result<String> {
        match self {
            Self::Stub(inner) => inner.finalize_action(planned_summary, goal, task),
            Self::Claude(inner) => inner.finalize_action(planned_summary, goal, task),
        }
    }
}

impl AgentBackend for StubBackend {
    fn name(&self) -> &'static str {
        "stub"
    }

    fn plan_action(&self, goal: Option<&Goal>, task: Option<&str>) -> Result<String> {
        let summary = if let Some(task) = task {
            format!("Stub backend accepted task \"{task}\" for deferred execution.")
        } else if let Some(goal) = goal {
            format!(
                "Stub backend prepared goal {}: {} with safe internal maintenance only.",
                goal.id, goal.title
            )
        } else {
            "Stub backend performed idle maintenance because no task or open goal was available."
                .to_string()
        };

        Ok(summary)
    }

    fn finalize_action(
        &self,
        planned_summary: &str,
        _goal: Option<&Goal>,
        _task: Option<&str>,
    ) -> Result<String> {
        Ok(format!(
            "{planned_summary} Act phase completed without external side effects."
        ))
    }
}
