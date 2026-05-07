//! Event triggers — webhook → tool execution without LLM.
//!
//! Direct tool execution triggered by events/webhooks,
//! bypassing LLM inference entirely.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Event payload from webhook or other trigger.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    /// Event type (e.g., "github.push", "docker.start").
    pub event_type: String,
    /// Source of the event.
    pub source: String,
    /// Event payload.
    pub payload: serde_json::Value,
    /// Event timestamp.
    pub timestamp: i64,
}

/// Trigger mapping event to tool execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventTrigger {
    /// Event type pattern (wildcard supported).
    pub event_pattern: String,
    /// Tool to execute.
    pub tool_name: String,
    /// Parameter mapping from event to tool args.
    pub param_mapping: HashMap<String, String>,
    /// Conditions for execution.
    pub conditions: Vec<TriggerCondition>,
    /// Transform the payload before tool call.
    pub transform: Option<serde_json::Value>,
}

/// Condition for trigger execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TriggerCondition {
    /// Field equals value.
    Equals { field: String, value: serde_json::Value },
    /// Field contains value.
    Contains { field: String, value: String },
    /// Regex match.
    Regex { field: String, pattern: String },
}

/// Event trigger router.
#[derive(Debug, Clone, Default)]
pub struct EventRouter {
    triggers: Vec<EventTrigger>,
}

impl EventRouter {
    /// Create new router.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a trigger rule.
    pub fn add_trigger(&mut self, trigger: EventTrigger) {
        self.triggers.push(trigger);
    }

    /// Route an event to a tool execution.
    pub fn route(&self, event: &Event) -> Option<ExecutingTool> {
        let matching = self.triggers.iter().find(|t| {
            // Simple pattern matching (could use glob crate)
            event.event_type == t.event_pattern || t.event_pattern == "*"
        })?;

        // Check conditions
        for condition in &matching.conditions {
            if !self.check_condition(condition, &event.payload) {
                return None;
            }
        }

        // Build tool args from event payload
        let mut args = HashMap::new();
        for (param, field) in &matching.param_mapping {
            if let Some(value) = event.payload.get(field) {
                args.insert(param.clone(), value.clone());
            }
        }

        Some(ExecutingTool {
            tool_name: matching.tool_name.clone(),
            args,
        })
    }

    fn check_condition(&self, condition: &TriggerCondition, payload: &serde_json::Value) -> bool {
        match condition {
            TriggerCondition::Equals { field, value } => {
                payload.get(field).map_or(false, |v| v == value)
            }
            TriggerCondition::Contains { field, value } => {
                payload.get(field).and_then(|v| v.as_str()).map_or(false, |s| s.contains(value))
            }
            TriggerCondition::Regex { field, pattern } => {
                let re = regex::Regex::new(pattern);
                payload.get(field).and_then(|v| v.as_str()).map_or(false, |s| {
                    re.map_or(false, |re| re.is_match(s))
                })
            }
        }
    }
}

/// Tool ready for execution.
pub struct ExecutingTool {
    pub tool_name: String,
    pub args: HashMap<String, serde_json::Value>,
}

/// Webhook endpoint handler.
#[derive(Debug, Clone, Default)]
pub struct WebhookHandler {
    router: EventRouter,
    secret: Option<String>,
}

impl WebhookHandler {
    /// Create new handler.
    pub fn new(router: EventRouter) -> Self {
        Self {
            router,
            secret: None,
        }
    }

    /// Set webhook secret for verification.
    pub fn with_secret(mut self, secret: &str) -> Self {
        self.secret = Some(secret.to_string());
        self
    }

    /// Process incoming webhook.
    pub fn handle(&self, event_type: &str, source: &str, payload: serde_json::Value) -> Option<ExecutingTool> {
        let event = Event {
            event_type: event_type.to_string(),
            source: source.to_string(),
            payload,
            timestamp: chrono::Utc::now().timestamp(),
        };

        self.router.route(&event)
    }

    /// Verify webhook signature.
    pub fn verify(&self, _signature: &str, _body: &[u8]) -> bool {
        // Would verify HMAC signature
        self.secret.is_some()
    }
}