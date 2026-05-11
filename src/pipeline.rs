//! Tool pipeline — declarative tool composition and chaining.
//!
//! Define pipelines that chain multiple tool calls together, where
//! output from one step feeds as input to the next.
//!
//! ```toml
//! [[pipeline]]
//! name = "deploy"
//! steps = [
//!   { tool = "git_pull", args = { branch = "main" } },
//!   { tool = "run_tests" },
//!   { tool = "deploy", args = { env = "$prev.branch" } },
//! ]
//! on_failure = "rollback"
//! ```

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// A single step in a pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineStep {
    /// Tool to execute.
    pub tool: String,
    /// Arguments for the tool. Values starting with $ are variable references.
    /// - $prev.field — output from previous step
    /// - $input.field — original pipeline input
    /// - $env.VAR — environment variable
    pub args: HashMap<String, String>,
    /// Optional condition: skip this step if condition is false.
    pub condition: Option<String>,
    /// Timeout in seconds for this step.
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
}

fn default_timeout() -> u64 {
    300
}

/// What to do when a step fails.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum FailurePolicy {
    /// Stop the pipeline.
    #[default]
    Stop,
    /// Continue to next step.
    Continue,
    /// Run the named rollback pipeline.
    Rollback(String),
    /// Retry up to N times.
    Retry {
        max_attempts: u32,
        delay_secs: u64,
    },
}

/// A declarative tool pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pipeline {
    /// Pipeline name.
    pub name: String,
    /// Ordered list of steps.
    pub steps: Vec<PipelineStep>,
    /// What to do on failure.
    #[serde(default)]
    pub on_failure: FailurePolicy,
    /// Description of the pipeline.
    pub description: Option<String>,
}

/// Result of a single step execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepResult {
    /// Step index in the pipeline.
    pub step_index: usize,
    /// Tool name that was executed.
    pub tool: String,
    /// Whether the step succeeded.
    pub success: bool,
    /// Output from the tool (JSON).
    pub output: serde_json::Value,
    /// Duration in milliseconds.
    pub duration_ms: u64,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Result of a full pipeline execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineResult {
    /// Pipeline name.
    pub name: String,
    /// Whether the overall pipeline succeeded.
    pub success: bool,
    /// Results from each step.
    pub steps: Vec<StepResult>,
    /// Total duration in milliseconds.
    pub total_duration_ms: u64,
}

/// Pipeline execution context with variable resolution.
#[derive(Debug, Clone)]
pub struct PipelineContext {
    /// Original input to the pipeline.
    pub input: HashMap<String, serde_json::Value>,
    /// Output from each completed step.
    pub step_outputs: Vec<serde_json::Value>,
}

impl PipelineContext {
    pub fn new(input: HashMap<String, serde_json::Value>) -> Self {
        Self {
            input,
            step_outputs: Vec::new(),
        }
    }

    /// Resolve a variable reference.
    /// - $prev.field — output from previous step
    /// - $input.field — original pipeline input
    /// - $env.VAR — environment variable
    pub fn resolve(&self, value: &str) -> String {
        if !value.starts_with('$') {
            return value.to_string();
        }

        let value = &value[1..]; // strip $

        // $prev.field
        if let Some(field) = value.strip_prefix("prev.") {
            if let Some(last) = self.step_outputs.last() {
                return last.get(field).and_then(|v| v.as_str()).unwrap_or("").to_string();
            }
            return String::new();
        }

        // $input.field
        if let Some(field) = value.strip_prefix("input.") {
            return self.input.get(field).and_then(|v| v.as_str()).unwrap_or("").to_string();
        }

        // $env.VAR
        if let Some(var) = value.strip_prefix("env.") {
            return std::env::var(var).unwrap_or_default();
        }

        String::new()
    }

    /// Resolve all args in a step, replacing variable references.
    pub fn resolve_args(
        &self,
        args: &HashMap<String, String>,
    ) -> HashMap<String, serde_json::Value> {
        args.iter()
            .map(|(k, v)| {
                let resolved = self.resolve(v);
                // Try to parse as JSON, otherwise use as string
                let json_val = serde_json::from_str::<serde_json::Value>(&resolved)
                    .unwrap_or(serde_json::Value::String(resolved));
                (k.clone(), json_val)
            })
            .collect()
    }

    /// Record the output of a step.
    pub fn record_output(&mut self, output: serde_json::Value) {
        self.step_outputs.push(output);
    }
}

/// Pipeline registry and executor.
#[derive(Debug, Clone, Default)]
pub struct PipelineRegistry {
    pipelines: HashMap<String, Pipeline>,
}

impl PipelineRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a pipeline.
    pub fn register(&mut self, pipeline: Pipeline) {
        self.pipelines.insert(pipeline.name.clone(), pipeline);
    }

    /// Get a pipeline by name.
    pub fn get(&self, name: &str) -> Option<&Pipeline> {
        self.pipelines.get(name)
    }

    /// List all registered pipeline names.
    pub fn list(&self) -> Vec<String> {
        self.pipelines.keys().cloned().collect()
    }

    /// Remove a pipeline.
    pub fn remove(&mut self, name: &str) -> bool {
        self.pipelines.remove(name).is_some()
    }

    /// Count of registered pipelines.
    pub fn len(&self) -> usize {
        self.pipelines.len()
    }

    /// Whether registry is empty.
    pub fn is_empty(&self) -> bool {
        self.pipelines.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_resolve_prev() {
        let mut ctx = PipelineContext::new(HashMap::new());
        ctx.step_outputs.push(serde_json::json!({ "branch": "main", "sha": "abc123" }));
        ctx.step_outputs.push(serde_json::json!({ "status": "passed" }));

        assert_eq!(ctx.resolve("$prev.status"), "passed");
    }

    #[test]
    fn test_context_resolve_input() {
        let mut input = HashMap::new();
        input.insert("repo".to_string(), serde_json::json!("praxis"));
        let ctx = PipelineContext::new(input);

        assert_eq!(ctx.resolve("$input.repo"), "praxis");
    }

    #[test]
    fn test_context_resolve_plain() {
        let ctx = PipelineContext::new(HashMap::new());
        assert_eq!(ctx.resolve("literal_value"), "literal_value");
    }

    #[test]
    fn test_context_resolve_args() {
        let mut ctx = PipelineContext::new(HashMap::new());
        ctx.step_outputs.push(serde_json::json!({ "path": "/dist" }));

        let args = HashMap::from([
            ("source".to_string(), "$prev.path".to_string()),
            ("format".to_string(), "mp3".to_string()),
        ]);

        let resolved = ctx.resolve_args(&args);
        assert_eq!(resolved["source"], "/dist");
        assert_eq!(resolved["format"], "mp3");
    }

    #[test]
    fn test_pipeline_registration() {
        let mut registry = PipelineRegistry::new();
        let pipeline = Pipeline {
            name: "deploy".to_string(),
            steps: vec![
                PipelineStep {
                    tool: "build".to_string(),
                    args: HashMap::new(),
                    condition: None,
                    timeout_secs: 120,
                },
                PipelineStep {
                    tool: "test".to_string(),
                    args: HashMap::new(),
                    condition: None,
                    timeout_secs: 300,
                },
            ],
            on_failure: FailurePolicy::Stop,
            description: Some("Build and deploy".to_string()),
        };

        registry.register(pipeline);
        assert_eq!(registry.len(), 1);
        assert!(registry.get("deploy").is_some());
        assert!(registry.get("nonexistent").is_none());
    }

    #[test]
    fn test_pipeline_serialization() {
        let pipeline = Pipeline {
            name: "ci".to_string(),
            steps: vec![PipelineStep {
                tool: "test".to_string(),
                args: HashMap::new(),
                condition: Some("$prev.success".to_string()),
                timeout_secs: 60,
            }],
            on_failure: FailurePolicy::Retry { max_attempts: 3, delay_secs: 5 },
            description: None,
        };

        let json = serde_json::to_string(&pipeline).unwrap();
        let back: Pipeline = serde_json::from_str(&json).unwrap();
        assert_eq!(back.name, "ci");
        assert_eq!(back.steps.len(), 1);
        assert_eq!(back.on_failure, FailurePolicy::Retry { max_attempts: 3, delay_secs: 5 });
    }

    #[test]
    fn test_failure_policy_default() {
        assert_eq!(FailurePolicy::default(), FailurePolicy::Stop);
    }
}
