//! Rule-based agent backend for deterministic operations.
//!
//! Zero-LLM mode where agents execute pure logic rules,
//! conditional trees, and deterministic workflows.

use anyhow::Result;
use serde::{Deserialize, Serialize};

use super::{AgentBackend, BackendOutput};
use crate::identity::Goal;

/// Rule condition for execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Condition {
    /// Check if a field equals a value.
    Equals {
        field: String,
        value: serde_json::Value,
    },
    /// Check if a field contains a substring.
    Contains { field: String, value: String },
    /// Check if numeric comparison.
    GreaterThan { field: String, value: f64 },
    /// Check if regex matches (compiled once, cached).
    Regex { field: String, pattern: String },
    /// Always true.
    Always,
    /// Logical AND of conditions.
    And(Vec<Condition>),
    /// Logical OR of conditions.
    Or(Vec<Condition>),
}

// W13 fix: Cache compiled regex patterns to avoid recompilation on every evaluation
use std::cell::RefCell;
use std::collections::HashMap;

thread_local! {
    static REGEX_CACHE: RefCell<HashMap<String, regex::Regex>> = RefCell::new(HashMap::new());
}

impl Condition {
    /// Evaluate condition against context.
    pub fn eval(&self, ctx: &serde_json::Value) -> bool {
        match self {
            Condition::Equals { field, value } => ctx.get(field) == Some(value),
            Condition::Contains { field, value } => {
                ctx.get(field).and_then(|v| v.as_str()).is_some_and(|s| s.contains(value))
            }
            Condition::GreaterThan { field, value } => {
                ctx.get(field).and_then(|v| v.as_f64()).is_some_and(|n| n > *value)
            }
            Condition::Regex { field, pattern } => {
                // W13 fix: Use cached compiled regex
                let field_val = ctx.get(field).and_then(|v| v.as_str());
                if let Some(text) = field_val {
                    return REGEX_CACHE.with(|cache| {
                        let mut cache = cache.borrow_mut();
                        let re = cache.entry(pattern.clone()).or_insert_with(|| {
                            regex::Regex::new(pattern)
                                .unwrap_or_else(|_| regex::Regex::new(".*").unwrap())
                        });
                        re.is_match(text)
                    });
                }
                false
            }
            Condition::Always => true,
            Condition::And(conditions) => conditions.iter().all(|c| c.eval(ctx)),
            Condition::Or(conditions) => conditions.iter().any(|c| c.eval(ctx)),
        }
    }
}

/// Action to execute when rule matches.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Action {
    /// Run a tool by name with parameters.
    Tool {
        name: String,
        args: serde_json::Value,
    },
    /// Set a variable in context.
    Set {
        field: String,
        value: serde_json::Value,
    },
    /// Send a message.
    Message { channel: String, text: String },
    /// Create a new context branch.
    Branch { name: String },
    /// Stop rule evaluation.
    Stop,
}

/// A rule with condition and action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    pub id: String,
    pub name: String,
    pub condition: Condition,
    pub action: Action,
    pub priority: i32,
}

/// Rule-based engine for deterministic execution.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RuleEngine {
    pub rules: Vec<Rule>,
    pub context: serde_json::Value,
}

impl RuleEngine {
    /// Create new empty rule engine.
    pub fn new() -> Self {
        Self {
            rules: vec![],
            context: serde_json::json!({}),
        }
    }

    /// Add a rule to the engine.
    pub fn add_rule(&mut self, rule: Rule) {
        self.rules.push(rule);
        self.rules.sort_by_key(|r| -r.priority);
    }

    /// Execute rules against current context.
    pub fn execute(&mut self) -> Vec<Action> {
        let mut executed = vec![];
        for rule in &self.rules {
            if rule.condition.eval(&self.context) {
                executed.push(rule.action.clone());
                if matches!(rule.action, Action::Stop) {
                    break;
                }
            }
        }
        executed
    }

    /// Update context with new values.
    pub fn update_context(&mut self, updates: serde_json::Value) {
        if let serde_json::Value::Object(map) = updates
            && let serde_json::Value::Object(ctx) = &mut self.context
        {
            for (k, v) in map {
                ctx.insert(k, v);
            }
        }
    }
}

/// Stub backend for zero-LLM mode.
pub struct RuleBackend;

/// S3 fix: RuleBackend implements AgentBackend for zero-LLM mode.
impl AgentBackend for RuleBackend {
    fn name(&self) -> &'static str {
        "rule"
    }

    fn answer_prompt(&self, prompt: &str) -> Result<BackendOutput> {
        Ok(BackendOutput {
            summary: Self::process(prompt),
            attempts: vec![],
        })
    }

    fn plan_action(
        &self,
        _goal: Option<&Goal>,
        _task: Option<&str>,
        _context: Option<&str>,
    ) -> Result<BackendOutput> {
        Ok(BackendOutput {
            summary: "Zero-LLM mode: no external planning available".to_string(),
            attempts: vec![],
        })
    }

    fn finalize_action(
        &self,
        summary: &str,
        _goal: Option<&Goal>,
        _task: Option<&str>,
        _context: Option<&str>,
    ) -> Result<BackendOutput> {
        Ok(BackendOutput {
            summary: summary.to_string(),
            attempts: vec![],
        })
    }
}

impl RuleBackend {
    /// Process input using rule engine.
    pub fn process(input: &str) -> String {
        // Simple rule-based response
        let rules = vec![
            ("hello", "Hello! How can I help?"),
            ("status", "Running in rule-based mode."),
            ("help", "Available: hello, status, time, date"),
        ];

        let input_lower = input.to_lowercase();
        for (pattern, response) in rules {
            if input_lower.contains(pattern) {
                return response.to_string();
            }
        }
        "I'm operating in deterministic mode. Try: hello, status, time, or date.".to_string()
    }

    /// Get current time string.
    pub fn time() -> String {
        chrono::Local::now().format("%H:%M:%S").to_string()
    }

    /// Get current date string.
    pub fn date() -> String {
        chrono::Local::now().format("%Y-%m-%d").to_string()
    }
}
