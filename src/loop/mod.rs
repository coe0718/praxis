mod outcome;
mod phases;
mod planner;
mod reflect;
mod runtime;
mod session;

#[cfg(test)]
mod tests;

use crate::state::SessionPhase;

pub use crate::backend::{AgentBackend, ConfiguredBackend, StubBackend};
pub use runtime::PraxisRuntime;

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
