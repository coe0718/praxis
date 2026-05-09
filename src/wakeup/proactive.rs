//! Proactive scheduling extension for wakeup module.
//!
//! Agent can schedule wake-ups based on conditions without external triggers.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Condition to monitor for proactive triggers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Condition {
    /// Time-based condition (cron expression).
    Time { cron: String },
    /// State-based condition.
    State {
        key: String,
        expected: serde_json::Value,
    },
    /// File change condition.
    FileChanged { path: String },
    /// Composite AND condition.
    And(Vec<Condition>),
    /// Composite OR condition.
    Or(Vec<Condition>),
}

/// Proactive wake-up schedule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WakeUp {
    pub id: String,
    pub name: String,
    pub condition: Condition,
    pub action: WakeAction,
    /// Priority for execution order.
    pub priority: i32,
    /// Whether this is enabled.
    pub enabled: bool,
}

/// Action to take when condition is met.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WakeAction {
    /// Run a tool.
    RunTool {
        name: String,
        args: serde_json::Value,
    },
    /// Send a message.
    SendMessage { channel: String, text: String },
    /// Trigger a skill.
    RunSkill {
        name: String,
        params: serde_json::Value,
    },
    /// Start a routine.
    StartRoutine { name: String },
}

/// Proactive agent scheduler.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProactiveAgent {
    pub wake_ups: Vec<WakeUp>,
    pub last_check: HashMap<String, i64>,
}

impl ProactiveAgent {
    /// Create new proactive agent.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a wake-up schedule.
    pub fn add_wake_up(&mut self, wake_up: WakeUp) {
        self.wake_ups.push(wake_up);
        self.wake_ups.sort_by_key(|w| -w.priority);
    }

    /// Check all conditions and return matching wake-up IDs.
    pub fn check(&mut self) -> Vec<String> {
        let mut executed = vec![];
        let now = chrono::Utc::now().timestamp();

        for wake_up in &self.wake_ups {
            if !wake_up.enabled {
                continue;
            }

            let last = self.last_check.get(&wake_up.id).copied().unwrap_or(0);
            if now - last < 60 {
                continue;
            }

            if self.check_condition(&wake_up.condition) {
                executed.push(wake_up.id.clone());
                self.last_check.insert(wake_up.id.clone(), now);
            }
        }
        executed
    }

    fn check_condition(&self, condition: &Condition) -> bool {
        match condition {
            Condition::Time { cron: _ } => {
                // Would check cron schedule - placeholder
                false
            }
            Condition::State { key: _, expected: _ } => false,
            Condition::FileChanged { path } => std::path::Path::new(path).exists(),
            Condition::And(conditions) => conditions.iter().all(|c| self.check_condition(c)),
            Condition::Or(conditions) => conditions.iter().any(|c| self.check_condition(c)),
        }
    }
}
