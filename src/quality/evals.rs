use std::{fs, path::PathBuf, process::Command};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use crate::{
    paths::PraxisPaths,
    storage::{EvalSeverity, EvalStatus},
};

use super::json_files;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EvalDefinition {
    pub id: String,
    pub name: String,
    pub when: String,
    pub severity: EvalSeverity,
    pub scenario: String,
    pub expected_behavior: Vec<String>,
    pub forbidden_behavior: Vec<String>,
    #[serde(default)]
    pub relevant_memories: Vec<String>,
    pub verify_with: String,
    pub commands: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvalOutcome {
    pub eval_id: String,
    pub name: String,
    pub severity: EvalSeverity,
    pub status: EvalStatus,
    pub summary: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EvalSummary {
    pub passed: usize,
    pub failed: usize,
    pub skipped: usize,
    pub trust_failures: usize,
}

pub trait EvalRunner {
    fn validate(&self, paths: &PraxisPaths) -> Result<usize>;
    fn run(&self, paths: &PraxisPaths) -> Result<Vec<EvalOutcome>>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvalTrigger {
    Always,
    Canary,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct LocalEvalSuite;

impl EvalRunner for LocalEvalSuite {
    fn validate(&self, paths: &PraxisPaths) -> Result<usize> {
        let files = json_files(&paths.evals_dir)?;
        for path in &files {
            load_eval(path)?;
        }
        Ok(files.len())
    }

    fn run(&self, paths: &PraxisPaths) -> Result<Vec<EvalOutcome>> {
        self.run_trigger(paths, EvalTrigger::Always)
    }
}

impl LocalEvalSuite {
    pub fn run_trigger(
        &self,
        paths: &PraxisPaths,
        trigger: EvalTrigger,
    ) -> Result<Vec<EvalOutcome>> {
        let mut results = Vec::new();
        for path in json_files(&paths.evals_dir)? {
            let definition = load_eval(&path)?;
            if !should_run(&definition, trigger) {
                results.push(EvalOutcome {
                    eval_id: definition.id,
                    name: definition.name,
                    severity: definition.severity,
                    status: EvalStatus::Skipped,
                    summary: "Eval skipped because it is not configured for every session."
                        .to_string(),
                });
                continue;
            }

            let status = run_definition(paths, &definition)?;
            let summary = match status {
                EvalStatus::Passed => format!("Eval passed: {}", definition.name),
                EvalStatus::Failed => format!("Eval failed: {}", definition.name),
                EvalStatus::Skipped => format!("Eval skipped: {}", definition.name),
            };
            results.push(EvalOutcome {
                eval_id: definition.id,
                name: definition.name,
                severity: definition.severity,
                status,
                summary,
            });
        }

        Ok(results)
    }
}

fn should_run(definition: &EvalDefinition, trigger: EvalTrigger) -> bool {
    match trigger {
        EvalTrigger::Always => definition.when == "always",
        EvalTrigger::Canary => matches!(definition.when.as_str(), "always" | "canary"),
    }
}

pub fn summarize(results: &[EvalOutcome]) -> EvalSummary {
    let passed = results.iter().filter(|result| result.status == EvalStatus::Passed).count();
    let failed = results.iter().filter(|result| result.status == EvalStatus::Failed).count();
    let skipped = results.iter().filter(|result| result.status == EvalStatus::Skipped).count();
    let trust_failures = results
        .iter()
        .filter(|result| {
            result.status == EvalStatus::Failed && result.severity == EvalSeverity::TrustDamaging
        })
        .count();

    EvalSummary {
        passed,
        failed,
        skipped,
        trust_failures,
    }
}

fn load_eval(path: &PathBuf) -> Result<EvalDefinition> {
    let raw =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    let eval: EvalDefinition = serde_json::from_str(&raw)
        .with_context(|| format!("invalid eval JSON in {}", path.display()))?;

    if eval.id.trim().is_empty() || eval.name.trim().is_empty() {
        bail!("eval in {} must include id and name", path.display());
    }
    if !matches!(eval.when.as_str(), "always" | "manual" | "canary") {
        bail!("eval {} has unsupported trigger {}", eval.id, eval.when);
    }
    if eval.scenario.trim().is_empty() {
        bail!("eval {} must include a scenario", eval.id);
    }
    if eval.expected_behavior.is_empty() || eval.forbidden_behavior.is_empty() {
        bail!("eval {} must describe expected and forbidden behavior", eval.id);
    }
    if eval.verify_with != "shell" {
        bail!("eval {} only supports verify_with = \"shell\" right now", eval.id);
    }
    if eval.commands.is_empty() {
        bail!("eval {} must include at least one shell command", eval.id);
    }

    Ok(eval)
}

fn run_definition(paths: &PraxisPaths, definition: &EvalDefinition) -> Result<EvalStatus> {
    const ALLOWED_EVAL_COMMANDS: &[&str] = &[
        "git", "grep", "test", "diff", "wc", "cat", "echo", "ls", "find", "true", "false", "exit",
    ];
    for command in &definition.commands {
        // Only allow commands that start with a whitelisted prefix.
        let cmd_prefix = command.split_whitespace().next().unwrap_or("");
        let cmd_basename = cmd_prefix.split('/').next_back().unwrap_or("");
        if !ALLOWED_EVAL_COMMANDS.contains(&cmd_basename) {
            bail!("eval command rejected (not in allowlist): `{command}`");
        }
        let output = Command::new("/bin/sh")
            .arg("-lc")
            .arg(command)
            .current_dir(&paths.data_dir)
            .output()
            .with_context(|| format!("failed to execute eval command `{command}`"))?;
        if !output.status.success() {
            return Ok(EvalStatus::Failed);
        }
    }

    Ok(EvalStatus::Passed)
}
