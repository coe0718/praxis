//! Dry-run and replay system for tool execution.
//!
//! Records tool execution plans (tool name, parameters, expected outcome)
//! and can replay them in dry-run mode (validate without executing) or
//! re-execute them. Useful for testing, auditing, and pre-flight checks.

use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

/// A recorded step in an execution plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanStep {
    /// Sequence number (1-based).
    pub seq: u32,
    /// Tool name.
    pub tool: String,
    /// Parameters as key-value pairs.
    pub params: serde_json::Value,
    /// Expected outcome description.
    #[serde(default)]
    pub expected: Option<String>,
    /// Actual outcome (filled after execution).
    #[serde(default)]
    pub actual: Option<String>,
    /// Status: "pending", "dry_run", "executed", "failed", "skipped".
    pub status: String,
}

/// An execution plan — a sequence of tool invocations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPlan {
    /// Plan ID.
    pub id: String,
    /// Human-readable label.
    #[serde(default)]
    pub label: String,
    /// When the plan was created.
    pub created_at: String,
    /// Steps in the plan.
    pub steps: Vec<PlanStep>,
    /// Tags for categorization.
    #[serde(default)]
    pub tags: Vec<String>,
}

impl ExecutionPlan {
    /// Create a new empty plan.
    pub fn new(id: &str, label: &str) -> Self {
        Self {
            id: id.to_string(),
            label: label.to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
            steps: Vec::new(),
            tags: Vec::new(),
        }
    }

    /// Add a step to the plan.
    pub fn add_step(&mut self, tool: &str, params: serde_json::Value, expected: Option<&str>) {
        let seq = (self.steps.len() as u32) + 1;
        self.steps.push(PlanStep {
            seq,
            tool: tool.to_string(),
            params,
            expected: expected.map(|s| s.to_string()),
            actual: None,
            status: "pending".to_string(),
        });
    }

    /// Mark a step as dry-run validated.
    pub fn mark_dry_run(&mut self, seq: u32, note: &str) -> Result<()> {
        let step = self
            .steps
            .iter_mut()
            .find(|s| s.seq == seq)
            .ok_or_else(|| anyhow::anyhow!("step {} not found", seq))?;
        step.status = "dry_run".to_string();
        step.actual = Some(note.to_string());
        Ok(())
    }

    /// Mark a step as executed.
    pub fn mark_executed(&mut self, seq: u32, result: &str) -> Result<()> {
        let step = self
            .steps
            .iter_mut()
            .find(|s| s.seq == seq)
            .ok_or_else(|| anyhow::anyhow!("step {} not found", seq))?;
        step.status = "executed".to_string();
        step.actual = Some(result.to_string());
        Ok(())
    }

    /// Mark a step as failed.
    pub fn mark_failed(&mut self, seq: u32, error: &str) -> Result<()> {
        let step = self
            .steps
            .iter_mut()
            .find(|s| s.seq == seq)
            .ok_or_else(|| anyhow::anyhow!("step {} not found", seq))?;
        step.status = "failed".to_string();
        step.actual = Some(error.to_string());
        Ok(())
    }

    /// Save the plan to a JSON file.
    pub fn save(&self, dir: &Path) -> Result<PathBuf> {
        fs::create_dir_all(dir).with_context(|| format!("creating plans dir {}", dir.display()))?;
        let path = dir.join(format!("{}.json", self.id));
        let json = serde_json::to_string_pretty(self).context("serializing plan")?;
        fs::write(&path, json).with_context(|| format!("writing {}", path.display()))?;
        Ok(path)
    }

    /// Load a plan from a JSON file.
    pub fn load(path: &Path) -> Result<Self> {
        let raw =
            fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        serde_json::from_str(&raw).with_context(|| format!("parsing {}", path.display()))
    }

    /// Run a dry-run validation — checks parameter schemas without executing.
    /// Returns a summary of what would happen.
    pub fn dry_run(&self) -> DryRunResult {
        let mut result = DryRunResult {
            plan_id: self.id.clone(),
            total_steps: self.steps.len(),
            valid: 0,
            warnings: Vec::new(),
            errors: Vec::new(),
        };

        for step in &self.steps {
            // Validate tool name is not empty
            if step.tool.is_empty() {
                result.errors.push(format!("step {}: empty tool name", step.seq));
                continue;
            }

            // Validate params is an object or array
            match &step.params {
                serde_json::Value::Object(_)
                | serde_json::Value::Array(_)
                | serde_json::Value::String(_) => {
                    result.valid += 1;
                }
                serde_json::Value::Null => {
                    result
                        .warnings
                        .push(format!("step {}: null params for '{}'", step.seq, step.tool));
                    result.valid += 1;
                }
                _ => {
                    result.valid += 1;
                }
            }

            // Check for common dangerous patterns
            if let Some(obj) = step.params.as_object() {
                if let Some(cmd) = obj.get("command").and_then(|v| v.as_str()) {
                    if cmd.contains("rm -rf")
                        || cmd.contains("drop table")
                        || cmd.contains("format")
                    {
                        result.warnings.push(format!(
                            "step {}: potentially destructive command in '{}'",
                            step.seq, step.tool
                        ));
                    }
                }
            }
        }

        result
    }

    /// Summary string.
    pub fn summary(&self) -> String {
        let pending = self.steps.iter().filter(|s| s.status == "pending").count();
        let executed = self.steps.iter().filter(|s| s.status == "executed").count();
        let failed = self.steps.iter().filter(|s| s.status == "failed").count();
        format!(
            "Plan '{}' [{}]: {} steps ({} pending, {} done, {} failed)",
            self.label,
            self.id,
            self.steps.len(),
            pending,
            executed,
            failed
        )
    }
}

/// Result of a dry-run validation.
#[derive(Debug, Serialize, Deserialize)]
pub struct DryRunResult {
    pub plan_id: String,
    pub total_steps: usize,
    pub valid: usize,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

impl DryRunResult {
    pub fn is_ok(&self) -> bool {
        self.errors.is_empty()
    }

    pub fn summary(&self) -> String {
        let mut lines = vec![format!(
            "Dry-run for '{}': {}/{} steps valid",
            self.plan_id, self.valid, self.total_steps
        )];
        for w in &self.warnings {
            lines.push(format!("  ⚠ {w}"));
        }
        for e in &self.errors {
            lines.push(format!("  ✗ {e}"));
        }
        lines.join("\n")
    }
}

/// List all plans in a directory.
pub fn list_plans(dir: &Path) -> Result<Vec<String>> {
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut plans = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().map(|e| e == "json").unwrap_or(false) {
            if let Some(stem) = path.file_stem() {
                plans.push(stem.to_string_lossy().to_string());
            }
        }
    }
    plans.sort();
    Ok(plans)
}

/// CLI handler for `praxis plan`.
pub fn handle_plan(data_dir: Option<PathBuf>, args: super::PlanArgs) -> Result<String> {
    use crate::paths::{PraxisPaths, default_data_dir};

    let data_dir_path = data_dir.unwrap_or(default_data_dir()?);
    let paths = PraxisPaths::for_data_dir(data_dir_path);
    let plans_dir = paths.data_dir.join("plans");

    match args.command {
        super::PlanCommand::DryRun(dr_args) => {
            let path = plans_dir.join(format!("{}.json", dr_args.plan_id));
            if !path.exists() {
                bail!("plan '{}' not found", dr_args.plan_id);
            }
            let plan = ExecutionPlan::load(&path)?;
            let result = plan.dry_run();
            Ok(result.summary())
        }
        super::PlanCommand::List => {
            let plans = list_plans(&plans_dir)?;
            if plans.is_empty() {
                return Ok("no plans found.".to_string());
            }
            let mut lines = vec![format!("{} plan(s):", plans.len())];
            for id in &plans {
                let path = plans_dir.join(format!("{id}.json"));
                if let Ok(plan) = ExecutionPlan::load(&path) {
                    lines.push(format!("  {} — {}", id, plan.summary()));
                }
            }
            Ok(lines.join("\n"))
        }
        super::PlanCommand::Show(show_args) => {
            let path = plans_dir.join(format!("{}.json", show_args.plan_id));
            if !path.exists() {
                bail!("plan '{}' not found", show_args.plan_id);
            }
            let plan = ExecutionPlan::load(&path)?;
            let mut lines = vec![plan.summary()];
            for step in &plan.steps {
                lines.push(format!(
                    "  #{} [{}] {} ({}) {}",
                    step.seq,
                    step.status,
                    step.tool,
                    if step.params.is_object() {
                        let keys: Vec<_> =
                            step.params.as_object().map(|o| o.keys().collect()).unwrap_or_default();
                        format!("{:?}", keys)
                    } else {
                        step.params.to_string()
                    },
                    step.actual.as_deref().unwrap_or(""),
                ));
            }
            Ok(lines.join("\n"))
        }
        super::PlanCommand::Create(create_args) => {
            let mut plan = ExecutionPlan::new(&create_args.id, &create_args.label);
            // Parse steps from --step arguments (format: "tool:param1=val1,param2=val2")
            for step_spec in &create_args.step {
                let parts: Vec<&str> = step_spec.splitn(2, ':').collect();
                let tool = parts[0].to_string();
                let params = if parts.len() > 1 {
                    let mut map = serde_json::Map::new();
                    for pair in parts[1].split(',') {
                        let kv: Vec<&str> = pair.splitn(2, '=').collect();
                        if kv.len() == 2 {
                            map.insert(
                                kv[0].to_string(),
                                serde_json::Value::String(kv[1].to_string()),
                            );
                        }
                    }
                    serde_json::Value::Object(map)
                } else {
                    serde_json::Value::Object(serde_json::Map::new())
                };
                plan.add_step(&tool, params, None);
            }
            let path = plan.save(&plans_dir)?;
            Ok(format!("created plan '{}' → {}", plan.id, path.display()))
        }
        super::PlanCommand::Remove(remove_args) => {
            let path = plans_dir.join(format!("{}.json", remove_args.plan_id));
            if !path.exists() {
                bail!("plan '{}' not found", remove_args.plan_id);
            }
            fs::remove_file(&path)?;
            Ok(format!("removed plan '{}'", remove_args.plan_id))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_create_and_dry_run_plan() {
        let mut plan = ExecutionPlan::new("test-1", "Test Plan");
        plan.add_step(
            "file-read",
            serde_json::json!({"path": "/tmp/test.txt"}),
            Some("reads a file"),
        );
        plan.add_step("shell-exec", serde_json::json!({"command": "ls -la"}), None);

        assert_eq!(plan.steps.len(), 2);
        assert_eq!(plan.steps[0].tool, "file-read");

        let result = plan.dry_run();
        assert!(result.is_ok());
        assert_eq!(result.valid, 2);
    }

    #[test]
    fn test_destructive_command_warning() {
        let mut plan = ExecutionPlan::new("danger-1", "Dangerous Plan");
        plan.add_step("shell-exec", serde_json::json!({"command": "rm -rf /"}), None);

        let result = plan.dry_run();
        assert_eq!(result.warnings.len(), 1);
        assert!(result.warnings[0].contains("destructive"));
    }

    #[test]
    fn test_save_and_load() {
        let tmp = tempdir().unwrap();
        let mut plan = ExecutionPlan::new("save-test", "Save Test");
        plan.add_step("web-fetch", serde_json::json!({"url": "https://example.com"}), None);

        let path = plan.save(tmp.path()).unwrap();
        assert!(path.exists());

        let loaded = ExecutionPlan::load(&path).unwrap();
        assert_eq!(loaded.id, "save-test");
        assert_eq!(loaded.steps.len(), 1);
    }

    #[test]
    fn test_mark_executed() {
        let mut plan = ExecutionPlan::new("exec-1", "Exec Test");
        plan.add_step("test", serde_json::json!({}), None);
        plan.mark_executed(1, "success").unwrap();
        assert_eq!(plan.steps[0].status, "executed");
    }
}
