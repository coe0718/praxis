mod phases;
mod runtime;
mod session;

#[cfg(test)]
mod tests;

use anyhow::Result;

use crate::{identity::Goal, state::SessionPhase};

pub use runtime::PraxisRuntime;

pub trait AgentBackend {
    fn name(&self) -> &'static str;
    fn plan_action(&self, goal: Option<&Goal>, task: Option<&str>) -> Result<String>;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct StubBackend;

#[derive(Debug, Clone)]
pub struct RunOptions {
    pub once: bool,
    pub force: bool,
    pub task: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunSummary {
    pub outcome: String,
    pub phase: SessionPhase,
    pub resumed: bool,
    pub selected_goal_id: Option<String>,
    pub selected_goal_title: Option<String>,
    pub selected_task: Option<String>,
    pub action_summary: String,
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
}
