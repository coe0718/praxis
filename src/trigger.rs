//! Event triggers — webhook → tool execution without LLM.
//!
//! Direct tool execution triggered by events/webhooks,
//! scheduled (cron) triggers, and conditional chains.

use chrono::{Datelike, Timelike};
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
    Equals {
        field: String,
        value: serde_json::Value,
    },
    /// Field contains value.
    Contains { field: String, value: String },
    /// Regex match.
    Regex { field: String, pattern: String },
    /// Field is greater than value (numeric).
    GreaterThan { field: String, value: f64 },
    /// Field is less than value (numeric).
    LessThan { field: String, value: f64 },
    /// All conditions must match.
    And(Vec<TriggerCondition>),
    /// Any condition must match.
    Or(Vec<TriggerCondition>),
    /// Negate a condition.
    Not(Box<TriggerCondition>),
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

    /// Remove trigger by event pattern.
    pub fn remove_trigger(&mut self, event_pattern: &str) -> bool {
        let before = self.triggers.len();
        self.triggers.retain(|t| t.event_pattern != event_pattern);
        before != self.triggers.len()
    }

    /// Route an event to a tool execution.
    pub fn route(&self, event: &Event) -> Option<ExecutingTool> {
        let matching = self.triggers.iter().find(|t| {
            // Support wildcard and prefix matching
            t.event_pattern == "*"
                || event.event_type == t.event_pattern
                || (t.event_pattern.ends_with(".*")
                    && event.event_type.starts_with(&t.event_pattern[..t.event_pattern.len() - 1]))
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

    /// Route an event to ALL matching tools (not just first).
    pub fn route_all(&self, event: &Event) -> Vec<ExecutingTool> {
        self.triggers
            .iter()
            .filter(|t| {
                t.event_pattern == "*"
                    || event.event_type == t.event_pattern
                    || (t.event_pattern.ends_with(".*")
                        && event
                            .event_type
                            .starts_with(&t.event_pattern[..t.event_pattern.len() - 1]))
            })
            .filter(|t| t.conditions.iter().all(|c| self.check_condition(c, &event.payload)))
            .map(|matching| {
                let mut args = HashMap::new();
                for (param, field) in &matching.param_mapping {
                    if let Some(value) = event.payload.get(field) {
                        args.insert(param.clone(), value.clone());
                    }
                }
                ExecutingTool {
                    tool_name: matching.tool_name.clone(),
                    args,
                }
            })
            .collect()
    }

    fn check_condition(&self, condition: &TriggerCondition, payload: &serde_json::Value) -> bool {
        match condition {
            TriggerCondition::Equals { field, value } => payload.get(field) == Some(value),
            TriggerCondition::Contains { field, value } => {
                payload.get(field).and_then(|v| v.as_str()).is_some_and(|s| s.contains(value))
            }
            TriggerCondition::Regex { field, pattern } => {
                let re = regex::Regex::new(pattern);
                payload
                    .get(field)
                    .and_then(|v| v.as_str())
                    .is_some_and(|s| re.is_ok_and(|re| re.is_match(s)))
            }
            TriggerCondition::GreaterThan { field, value } => {
                payload.get(field).and_then(|v| v.as_f64()).is_some_and(|v| v > *value)
            }
            TriggerCondition::LessThan { field, value } => {
                payload.get(field).and_then(|v| v.as_f64()).is_some_and(|v| v < *value)
            }
            TriggerCondition::And(conditions) => {
                conditions.iter().all(|c| self.check_condition(c, payload))
            }
            TriggerCondition::Or(conditions) => {
                conditions.iter().any(|c| self.check_condition(c, payload))
            }
            TriggerCondition::Not(inner) => !self.check_condition(inner, payload),
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
        Self { router, secret: None }
    }

    /// Set webhook secret for verification.
    pub fn with_secret(mut self, secret: &str) -> Self {
        self.secret = Some(secret.to_string());
        self
    }

    /// Process incoming webhook.
    pub fn handle(
        &self,
        event_type: &str,
        source: &str,
        payload: serde_json::Value,
    ) -> Option<ExecutingTool> {
        let event = Event {
            event_type: event_type.to_string(),
            source: source.to_string(),
            payload,
            timestamp: chrono::Utc::now().timestamp(),
        };

        self.router.route(&event)
    }

    /// Verify webhook signature using HMAC-SHA256.
    pub fn verify(&self, signature: &str, body: &[u8]) -> bool {
        let Some(ref secret) = self.secret else {
            return false;
        };
        use hmac::{Hmac, Mac};
        type HmacSha256 = Hmac<sha2::Sha256>;

        let mut mac = match HmacSha256::new_from_slice(secret.as_bytes()) {
            Ok(m) => m,
            Err(_) => return false,
        };
        mac.update(body);
        let expected = mac.finalize().into_bytes();
        let expected_hex = hex::encode(expected);

        // Constant-time comparison
        let expected_bytes = match hex::decode(signature) {
            Ok(b) => b,
            Err(_) => return expected_hex == signature,
        };

        if expected_bytes.len() != expected.len() {
            return false;
        }

        // Simple constant-time comparison
        let mut diff = 0u8;
        for (a, b) in expected.iter().zip(expected_bytes.iter()) {
            diff |= a ^ b;
        }
        diff == 0
    }
}

// ── Scheduled Triggers ────────────────────────────────────────────────────────

/// A scheduled (cron) trigger.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledTrigger {
    /// Unique ID for this trigger.
    pub id: String,
    /// Cron expression (e.g., "0 9 * * *", "*/5 * * * *").
    pub cron: String,
    /// Tool to execute.
    pub tool_name: String,
    /// Static args for the tool.
    pub args: HashMap<String, serde_json::Value>,
    /// Optional event type to emit when triggered.
    pub emit_event: Option<String>,
    /// Timezone for cron evaluation.
    #[serde(default = "default_timezone")]
    pub timezone: String,
    /// Whether this trigger is enabled.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Description of what this trigger does.
    pub description: Option<String>,
}

fn default_timezone() -> String {
    "UTC".to_string()
}
fn default_true() -> bool {
    true
}

/// Manager for scheduled triggers.
#[derive(Debug, Clone, Default)]
pub struct ScheduledTriggerManager {
    triggers: Vec<ScheduledTrigger>,
}

impl ScheduledTriggerManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a scheduled trigger.
    pub fn add(&mut self, trigger: ScheduledTrigger) {
        self.triggers.push(trigger);
    }

    /// Remove a scheduled trigger by ID.
    pub fn remove(&mut self, id: &str) -> bool {
        let before = self.triggers.len();
        self.triggers.retain(|t| t.id != id);
        before != self.triggers.len()
    }

    /// Get all triggers.
    pub fn list(&self) -> &[ScheduledTrigger] {
        &self.triggers
    }

    /// Check if any trigger should fire at the given time.
    /// Returns list of tools that should execute.
    pub fn check(&self, now: chrono::DateTime<chrono::Utc>) -> Vec<ExecutingTool> {
        self.triggers
            .iter()
            .filter(|t| t.enabled && self.should_fire(t, now))
            .map(|t| ExecutingTool {
                tool_name: t.tool_name.clone(),
                args: t.args.clone(),
            })
            .collect()
    }

    /// Simple cron evaluation. Supports: minute hour day month weekday.
    /// Handles: *, */N, specific values, comma-separated values.
    fn should_fire(&self, trigger: &ScheduledTrigger, now: chrono::DateTime<chrono::Utc>) -> bool {
        let parts: Vec<&str> = trigger.cron.split_whitespace().collect();
        if parts.len() != 5 {
            log::warn!("trigger: invalid cron '{}', expected 5 fields", trigger.cron);
            return false;
        }

        let cron_fields: [i64; 5] = [
            now.minute() as i64,
            now.hour() as i64,
            now.day() as i64,
            now.month() as i64,
            // Convert chrono weekday (Mon=0) to cron weekday (Mon=1, Sun=7 or 0)
            now.weekday().num_days_from_monday() as i64 + 1,
        ];

        for (i, part) in parts.iter().enumerate() {
            if !self.cron_field_matches(part, cron_fields[i]) {
                return false;
            }
        }
        true
    }

    fn cron_field_matches(&self, field: &str, value: i64) -> bool {
        if field == "*" {
            return true;
        }
        if field == "?" {
            return true; // ? means "any" (used for day-of-week vs day-of-month)
        }
        if let Some(step) = field.strip_prefix("*/") {
            if let Ok(step) = step.parse::<i64>() {
                return step > 0 && value % step == 0;
            }
        }
        // Comma-separated values: "1,15"
        if field.contains(',') {
            return field.split(',').any(|v| v.parse::<i64>().map(|v| v == value).unwrap_or(false));
        }
        // Range: "1-5"
        if field.contains('-') {
            let parts: Vec<&str> = field.split('-').collect();
            if parts.len() == 2 {
                if let (Ok(start), Ok(end)) = (parts[0].parse::<i64>(), parts[1].parse::<i64>()) {
                    return value >= start && value <= end;
                }
            }
        }
        // Exact value
        field.parse::<i64>().map(|v| v == value).unwrap_or(false)
    }
}

// ── Trigger Chain ─────────────────────────────────────────────────────────────

/// A chain of triggers where output of one feeds into the next.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerChain {
    /// Chain ID.
    pub id: String,
    /// Ordered list of tool executions.
    pub steps: Vec<ChainStep>,
    /// Whether to stop on first failure.
    #[serde(default = "default_true")]
    pub stop_on_failure: bool,
}

/// A step in a trigger chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainStep {
    /// Tool to execute.
    pub tool_name: String,
    /// Arg mappings. Use "$prev.output.field" to reference previous step output.
    pub args: HashMap<String, String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_routing() {
        let mut router = EventRouter::new();
        router.add_trigger(EventTrigger {
            event_pattern: "github.push".to_string(),
            tool_name: "deploy".to_string(),
            param_mapping: HashMap::from([
                ("branch".to_string(), "ref".to_string()),
                ("repo".to_string(), "repository.full_name".to_string()),
            ]),
            conditions: vec![],
            transform: None,
        });

        let event = Event {
            event_type: "github.push".to_string(),
            source: "github".to_string(),
            payload: serde_json::json!({
                "ref": "refs/heads/main",
                "repository": { "full_name": "coe0718/praxis" }
            }),
            timestamp: 0,
        };

        let tool = router.route(&event).unwrap();
        assert_eq!(tool.tool_name, "deploy");
        assert_eq!(tool.args["branch"], "refs/heads/main");
    }

    #[test]
    fn test_wildcard_routing() {
        let mut router = EventRouter::new();
        router.add_trigger(EventTrigger {
            event_pattern: "docker.*".to_string(),
            tool_name: "log_event".to_string(),
            param_mapping: HashMap::new(),
            conditions: vec![],
            transform: None,
        });

        let event = Event {
            event_type: "docker.container.start".to_string(),
            source: "docker".to_string(),
            payload: serde_json::json!({}),
            timestamp: 0,
        };

        assert!(router.route(&event).is_some());
    }

    #[test]
    fn test_condition_equals() {
        let mut router = EventRouter::new();
        router.add_trigger(EventTrigger {
            event_pattern: "github.push".to_string(),
            tool_name: "deploy".to_string(),
            param_mapping: HashMap::new(),
            conditions: vec![TriggerCondition::Equals {
                field: "ref".to_string(),
                value: serde_json::json!("refs/heads/main"),
            }],
            transform: None,
        });

        let event_match = Event {
            event_type: "github.push".to_string(),
            source: "github".to_string(),
            payload: serde_json::json!({ "ref": "refs/heads/main" }),
            timestamp: 0,
        };
        let event_no_match = Event {
            event_type: "github.push".to_string(),
            source: "github".to_string(),
            payload: serde_json::json!({ "ref": "refs/heads/dev" }),
            timestamp: 0,
        };

        assert!(router.route(&event_match).is_some());
        assert!(router.route(&event_no_match).is_none());
    }

    #[test]
    fn test_condition_and_or() {
        let mut router = EventRouter::new();
        router.add_trigger(EventTrigger {
            event_pattern: "alert".to_string(),
            tool_name: "escalate".to_string(),
            param_mapping: HashMap::new(),
            conditions: vec![TriggerCondition::And(vec![
                TriggerCondition::GreaterThan {
                    field: "severity".to_string(),
                    value: 5.0,
                },
                TriggerCondition::Or(vec![
                    TriggerCondition::Equals {
                        field: "source".to_string(),
                        value: serde_json::json!("prod"),
                    },
                    TriggerCondition::Equals {
                        field: "source".to_string(),
                        value: serde_json::json!("staging"),
                    },
                ]),
            ])],
            transform: None,
        });

        let event_ok = Event {
            event_type: "alert".to_string(),
            source: "monitor".to_string(),
            payload: serde_json::json!({ "severity": 8, "source": "prod" }),
            timestamp: 0,
        };
        let event_low = Event {
            event_type: "alert".to_string(),
            source: "monitor".to_string(),
            payload: serde_json::json!({ "severity": 3, "source": "prod" }),
            timestamp: 0,
        };
        let event_wrong_source = Event {
            event_type: "alert".to_string(),
            source: "monitor".to_string(),
            payload: serde_json::json!({ "severity": 8, "source": "dev" }),
            timestamp: 0,
        };

        assert!(router.route(&event_ok).is_some());
        assert!(router.route(&event_low).is_none());
        assert!(router.route(&event_wrong_source).is_none());
    }

    #[test]
    fn test_route_all() {
        let mut router = EventRouter::new();
        router.add_trigger(EventTrigger {
            event_pattern: "*".to_string(),
            tool_name: "logger".to_string(),
            param_mapping: HashMap::new(),
            conditions: vec![],
            transform: None,
        });
        router.add_trigger(EventTrigger {
            event_pattern: "deploy".to_string(),
            tool_name: "notifier".to_string(),
            param_mapping: HashMap::new(),
            conditions: vec![],
            transform: None,
        });

        let event = Event {
            event_type: "deploy".to_string(),
            source: "ci".to_string(),
            payload: serde_json::json!({}),
            timestamp: 0,
        };

        let tools = router.route_all(&event);
        assert_eq!(tools.len(), 2);
    }

    #[test]
    fn test_scheduled_trigger_cron_match() {
        let mut mgr = ScheduledTriggerManager::new();
        mgr.add(ScheduledTrigger {
            id: "test".to_string(),
            cron: "30 9 * * 1".to_string(), // 9:30 every Monday (1=Mon in standard cron)
            tool_name: "weekly_report".to_string(),
            args: HashMap::new(),
            emit_event: None,
            timezone: "UTC".to_string(),
            enabled: true,
            description: Some("Weekly report".to_string()),
        });

        // 2026-05-11 is a Monday
        let monday = chrono::DateTime::parse_from_rfc3339("2026-05-11T09:30:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc);
        let tools = mgr.check(monday);
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].tool_name, "weekly_report");

        // 2026-05-12 is a Tuesday
        let tuesday = chrono::DateTime::parse_from_rfc3339("2026-05-12T09:30:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc);
        let tools = mgr.check(tuesday);
        assert!(tools.is_empty());
    }

    #[test]
    fn test_scheduled_trigger_step() {
        let mut mgr = ScheduledTriggerManager::new();
        mgr.add(ScheduledTrigger {
            id: "every5".to_string(),
            cron: "*/5 * * * *".to_string(),
            tool_name: "health_check".to_string(),
            args: HashMap::new(),
            emit_event: None,
            timezone: "UTC".to_string(),
            enabled: true,
            description: None,
        });

        // 0, 5, 10, 15... should match
        let t0 = chrono::DateTime::parse_from_rfc3339("2026-05-11T10:00:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc);
        let t5 = chrono::DateTime::parse_from_rfc3339("2026-05-11T10:05:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc);
        let t7 = chrono::DateTime::parse_from_rfc3339("2026-05-11T10:07:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc);

        assert_eq!(mgr.check(t0).len(), 1);
        assert_eq!(mgr.check(t5).len(), 1);
        assert!(mgr.check(t7).is_empty());
    }

    #[test]
    fn test_webhook_verify() {
        let handler = WebhookHandler::new(EventRouter::new()).with_secret("test_secret");
        let body = b"test body";

        // Correct signature
        use hmac::{Hmac, Mac};
        type HmacSha256 = Hmac<sha2::Sha256>;
        let mut mac = HmacSha256::new_from_slice(b"test_secret").unwrap();
        mac.update(body);
        let sig = hex::encode(mac.finalize().into_bytes());

        assert!(handler.verify(&sig, body));
        assert!(!handler.verify("wrong", body));
    }

    #[test]
    fn test_trigger_chain_serialization() {
        let chain = TriggerChain {
            id: "deploy_chain".to_string(),
            steps: vec![
                ChainStep {
                    tool_name: "build".to_string(),
                    args: HashMap::new(),
                },
                ChainStep {
                    tool_name: "test".to_string(),
                    args: HashMap::from([(
                        "artifact".to_string(),
                        "$prev.output.path".to_string(),
                    )]),
                },
            ],
            stop_on_failure: true,
        };
        let json = serde_json::to_string(&chain).unwrap();
        let back: TriggerChain = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, "deploy_chain");
        assert_eq!(back.steps.len(), 2);
    }
}
