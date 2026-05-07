//! Runtime Skill Creation — Agent creates its own skills at runtime.
//!
//! Allows Praxis to dynamically generate and register new skills during execution.

use serde::{Deserialize, Serialize};

/// A runtime-generated skill definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeSkill {
    pub id: String,
    pub name: String,
    pub description: String,
    pub trigger: SkillTrigger,
    pub actions: Vec<SkillAction>,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SkillTrigger {
    /// Trigger on message pattern match.
    Pattern(String),
    /// Trigger on tool invocation.
    Tool(String),
    /// Trigger on scheduled time.
    Cron(String),
    /// Trigger on event type.
    Event(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SkillAction {
    /// Run a tool.
    RunTool { name: String, args: serde_json::Value },
    /// Set context variable.
    Set { key: String, value: serde_json::Value },
    /// Send a message.
    Message { channel: String, text: String },
    /// Invoke another skill.
    InvokeSkill { id: String, params: serde_json::Value },
}

/// Runtime skill factory for creating skills dynamically.
pub struct RuntimeSkillFactory {
    skills: std::collections::HashMap<String, RuntimeSkill>,
}

impl RuntimeSkillFactory {
    pub fn new() -> Self {
        Self {
            skills: std::collections::HashMap::new(),
        }
    }

    /// Create a new skill from a template or specification.
    pub fn create(&mut self, spec: SkillSpec) -> Result<String, anyhow::Error> {
        let id = format!(
            "rt_{}",
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
        );

        let skill = RuntimeSkill {
            id: id.clone(),
            name: spec.name,
            description: spec.description,
            trigger: spec.trigger,
            actions: spec.actions,
            created_at: chrono::Utc::now().timestamp(),
        };

        self.skills.insert(id.clone(), skill);
        Ok(id)
    }

    /// Get a skill by ID.
    pub fn get(&self, id: &str) -> Option<&RuntimeSkill> {
        self.skills.get(id)
    }

    /// List all runtime skills.
    pub fn list(&self) -> Vec<&RuntimeSkill> {
        self.skills.values().collect()
    }

    /// Remove a runtime skill.
    pub fn remove(&mut self, id: &str) -> bool {
        self.skills.remove(id).is_some()
    }
}

impl Default for RuntimeSkillFactory {
    fn default() -> Self {
        Self::new()
    }
}

/// Specification for creating a new skill.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillSpec {
    pub name: String,
    pub description: String,
    pub trigger: SkillTrigger,
    pub actions: Vec<SkillAction>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skill_creation() {
        let mut factory = RuntimeSkillFactory::new();

        let spec = SkillSpec {
            name: "test-skill".into(),
            description: "A test skill".into(),
            trigger: SkillTrigger::Pattern("hello".into()),
            actions: vec![SkillAction::Message {
                channel: "test".into(),
                text: "world".into(),
            }],
        };

        let id = factory.create(spec).unwrap();
        assert!(factory.get(&id).is_some());
    }
}