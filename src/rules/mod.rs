//! Rule-Based Mode — Deterministic agent behavior without LLM calls.
//!
//! Matches patterns against predefined rules for known/scripted tasks.
//! Saves API costs for routine operations.
//!
use serde::{Deserialize, Serialize};

/// A rule condition.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Condition {
    /// Field equals value.
    Equals {
        field: String,
        value: serde_json::Value,
    },
    /// Field contains substring.
    Contains { field: String, value: String },
    /// Regex match.
    Regex { field: String, pattern: String },
    /// Always true.
    Always,
}

/// A rule action.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Action {
    /// Run a tool.
    Tool {
        name: String,
        args: serde_json::Value,
    },
    /// Set context variable.
    Set {
        field: String,
        value: serde_json::Value,
    },
    /// Respond with message.
    Message { text: String },
    /// Branch to another rule.
    Branch { rule: String },
}

/// A rule definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    /// Unique rule identifier.
    pub id: String,
    /// Human-readable description.
    pub description: String,
    /// Conditions that must all be true.
    pub when: Vec<Condition>,
    /// Actions to execute.
    pub then: Vec<Action>,
    /// Priority (higher = checked first).
    #[serde(default)]
    pub priority: i32,
}

/// Rule set configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RuleSet {
    /// Rules in this set.
    pub rules: Vec<Rule>,
}

/// Rule engine for pattern matching.
pub struct RuleEngine {
    ruleset: RuleSet,
}

impl RuleEngine {
    pub fn new(ruleset: RuleSet) -> Self {
        Self { ruleset }
    }

    /// Load ruleset from YAML/JSON.
    pub fn from_yaml(yaml: &str) -> Result<Self, serde_yaml::Error> {
        let ruleset: RuleSet = serde_yaml::from_str(yaml)?;
        Ok(Self::new(ruleset))
    }

    /// Find first matching rule.
    pub fn match_rule(&self, context: &serde_json::Value) -> Option<&Rule> {
        let mut sorted = self.ruleset.rules.iter().collect::<Vec<_>>();
        sorted.sort_by_key(|a| std::cmp::Reverse(a.priority));

        sorted
            .into_iter()
            .find(|&rule| self.evaluate_conditions(&rule.when, context))
            .map(|v| v as _)
    }

    fn evaluate_conditions(&self, conditions: &[Condition], context: &serde_json::Value) -> bool {
        conditions.iter().all(|c| self.evaluate_condition(c, context))
    }

    fn evaluate_condition(&self, condition: &Condition, context: &serde_json::Value) -> bool {
        match condition {
            Condition::Equals { field, value } => context.get(field) == Some(value),
            Condition::Contains { field, value } => {
                context.get(field).and_then(|v| v.as_str()).is_some_and(|s| s.contains(value))
            }
            Condition::Regex { field, pattern } => {
                let re = regex::Regex::new(pattern);
                context
                    .get(field)
                    .and_then(|v| v.as_str())
                    .is_some_and(|s| re.is_ok_and(|re| re.is_match(s)))
            }
            Condition::Always => true,
        }
    }
}

/// Execution result from a rule match.
#[derive(Debug, Clone)]
pub struct RuleResult {
    pub matched_rule: Option<String>,
    pub actions: Vec<Action>,
}

impl RuleResult {
    pub fn matched(rule_id: &str, actions: Vec<Action>) -> Self {
        Self {
            matched_rule: Some(rule_id.to_string()),
            actions,
        }
    }

    pub fn no_match() -> Self {
        Self {
            matched_rule: None,
            actions: Vec::new(),
        }
    }
}

/// Zero-LLM configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZeroLLMConfig {
    /// Enable rule-based mode.
    #[serde(default)]
    pub enabled: bool,
    /// Path to rules file.
    #[serde(default)]
    pub rules_path: String,
    /// Fallback to LLM if no rule matches.
    #[serde(default = "default_fallback")]
    pub fallback_to_llm: bool,
}

fn default_fallback() -> bool {
    true
}

impl Default for ZeroLLMConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            rules_path: "rules.yaml".to_string(),
            fallback_to_llm: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rule_match() {
        let yaml = r#"
rules:
  - id: greet
    description: Greet the user
    priority: 10
    when:
      - type: Equals
        field: intent
        value: greet
    then:
      - type: Message
        text: "Hello! How can I help you today?"
"#;
        let engine = RuleEngine::from_yaml(yaml).unwrap();
        let context = serde_json::json!({"intent": "greet"});
        let rule = engine.match_rule(&context);
        assert!(rule.is_some());
        assert_eq!(rule.unwrap().id, "greet");
    }
}
