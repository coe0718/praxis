//! Proactive agent wake-up scheduler and condition monitoring.
//!
//! Agent can schedule wake-ups, monitor conditions, and initiate
//! actions without external prompts.

use serde::{Deserialize, Serialize};

/// Condition to monitor for proactive triggers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Condition {
    /// Time-based condition.
    Time { schedule: cron::Schedule },
    /// State-based condition.
    State { key: String, expected: serde_json::Value },
    /// File change condition.
    FileChanged { path: String },
    /// Webhook trigger condition.
    Webhook { endpoint: String },
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
    RunTool { name: String, args: serde_json::Value },
    /// Send a message.
    SendMessage { channel: String, text: String },
    /// Trigger a skill.
    RunSkill { name: String, params: serde_json::Value },
    /// Start a routine.
    StartRoutine { name: String },
}

/// Proactive agent scheduler.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProactiveAgent {
    pub wake_ups: Vec<WakeUp>,
    pub last_check: std::collections::HashMap<String, i64>,
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

    /// Check all conditions and execute actions.
    pub async fn check(&mut self) -> Vec<String> {
        let mut executed = vec![];
        let now = chrono::Utc::now().timestamp();

        for wake_up in &self.wake_ups {
            if !wake_up.enabled {
                continue;
            }

            // Check last execution time
            let last = self.last_check.get(&wake_up.id).copied().unwrap_or(0);
            if now - last < 60 {
                continue; // Don't run more than once per minute
            }

            if self.check_condition(&wake_up.condition).await {
                // Execute action
                self.execute_action(&wake_up.action).await;
                executed.push(wake_up.id.clone());
                self.last_check.insert(wake_up.id.clone(), now);
            }
        }

        executed
    }

    async fn check_condition(&self, condition: &Condition) -> bool {
        match condition {
            Condition::Time { schedule } => {
                schedule.upcoming(chrono::Utc::now()).next().is_some()
            }
            Condition::State { key: _, expected: _ } => {
                // Would check actual state
                false
            }
            Condition::FileChanged { path } => {
                // Would check file mtime
                std::path::Path::new(path).exists()
            }
            Condition::Webhook { endpoint: _ } => false,
            Condition::And(conditions) => {
                conditions.iter().all(|c| futures::executor::block_on(self.check_condition(c)))
            }
            Condition::Or(conditions) => {
                conditions.iter().any(|c| futures::executor::block_on(self.check_condition(c)))
            }
        }
    }

    async fn execute_action(&self, action: &WakeAction) {
        match action {
            WakeAction::RunTool { name: _, args: _ } => {
                // Would execute tool
            }
            WakeAction::SendMessage { channel: _, text: _ } => {
                // Would send message
            }
            WakeAction::RunSkill { name: _, params: _ } => {
                // Would run skill
            }
            WakeAction::StartRoutine { name: _ } => {
                // Would start routine
            }
        }
    }
}

/// Run proactive agent loop.
pub async fn run_proactive_loop(mut agent: ProactiveAgent) {
    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));
    loop {
        interval.tick().await;
        agent.check().await;
    }
}