use std::{fs, path::PathBuf, process::Command};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use crate::{paths::PraxisPaths, storage::ReviewStatus};

use super::json_files;

/// Hard ceiling for reviewer sub-agent resource usage.
///
/// Design spec: the smaller of 15% of the primary session context budget or
/// 8 000 tokens.  Since we use shell-based review rather than an LLM reviewer
/// sub-agent, this manifests as limits on command count and captured output.
pub struct ReviewerBudget {
    /// Maximum number of shell commands to run.
    pub max_commands: usize,
    /// Maximum bytes captured per command (stdout + stderr combined).
    pub max_output_bytes: usize,
}

impl Default for ReviewerBudget {
    fn default() -> Self {
        Self {
            max_commands: 20,
            max_output_bytes: 2_048,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GoalCriteria {
    pub goal_id: String,
    pub done_when: Vec<String>,
    pub verify_with: String,
    pub commands: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewOutcome {
    pub status: ReviewStatus,
    pub summary: String,
    pub findings: Vec<String>,
}

pub trait Reviewer {
    fn validate(&self, paths: &PraxisPaths) -> Result<usize>;
    fn review(&self, paths: &PraxisPaths, goal_id: Option<&str>) -> Result<ReviewOutcome>;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct LocalReviewer;

impl Reviewer for LocalReviewer {
    fn validate(&self, paths: &PraxisPaths) -> Result<usize> {
        let files = json_files(&paths.goals_criteria_dir)?;
        for path in &files {
            load_criteria(path)?;
        }
        Ok(files.len())
    }

    fn review(&self, paths: &PraxisPaths, goal_id: Option<&str>) -> Result<ReviewOutcome> {
        self.review_with_budget(paths, goal_id, &ReviewerBudget::default())
    }
}

impl LocalReviewer {
    /// Review with an explicit resource budget.
    pub fn review_with_budget(
        &self,
        paths: &PraxisPaths,
        goal_id: Option<&str>,
        budget: &ReviewerBudget,
    ) -> Result<ReviewOutcome> {
        let Some(goal_id) = goal_id else {
            return Ok(ReviewOutcome {
                status: ReviewStatus::Skipped,
                summary: "Reviewer skipped because no goal was selected.".to_string(),
                findings: Vec::new(),
            });
        };

        let Some(criteria) = load_goal_criteria(paths, goal_id)? else {
            return Ok(ReviewOutcome {
                status: ReviewStatus::Passed,
                summary: format!("Reviewer passed {goal_id} with no criteria file."),
                findings: Vec::new(),
            });
        };

        if criteria.commands.len() > budget.max_commands {
            bail!(
                "Reviewer budget exceeded: criteria {} has {} commands but the budget allows {}",
                goal_id,
                criteria.commands.len(),
                budget.max_commands
            );
        }

        let mut findings = Vec::new();
        for command in &criteria.commands {
            if let Some(failure) = run_shell_bounded(paths, command, budget.max_output_bytes)? {
                findings.push(failure);
            }
        }

        let status = if findings.is_empty() {
            ReviewStatus::Passed
        } else {
            ReviewStatus::Failed
        };
        let summary = match status {
            ReviewStatus::Passed => format!(
                "Reviewer passed {} using {} shell checks.",
                criteria.goal_id,
                criteria.commands.len()
            ),
            ReviewStatus::Failed => format!(
                "Reviewer failed {} with {} finding(s).",
                criteria.goal_id,
                findings.len()
            ),
            ReviewStatus::Skipped => format!("Reviewer skipped {}.", criteria.goal_id),
        };

        Ok(ReviewOutcome {
            status,
            summary,
            findings,
        })
    }
}

pub fn load_goal_criteria(paths: &PraxisPaths, goal_id: &str) -> Result<Option<GoalCriteria>> {
    let path = criteria_path(paths, goal_id);
    if !path.exists() {
        return Ok(None);
    }

    load_criteria(&path).map(Some)
}

fn criteria_path(paths: &PraxisPaths, goal_id: &str) -> PathBuf {
    paths.goals_criteria_dir.join(format!("{goal_id}.json"))
}

fn load_criteria(path: &PathBuf) -> Result<GoalCriteria> {
    let raw =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    let criteria: GoalCriteria = serde_json::from_str(&raw)
        .with_context(|| format!("invalid criteria JSON in {}", path.display()))?;

    if criteria.goal_id.trim().is_empty() {
        bail!("criteria in {} must include a goal_id", path.display());
    }
    if criteria.done_when.is_empty() {
        bail!(
            "criteria in {} must include at least one done_when item",
            path.display()
        );
    }
    if criteria.verify_with != "shell" {
        bail!(
            "criteria in {} only supports verify_with = \"shell\" right now",
            path.display()
        );
    }
    if criteria.commands.is_empty() {
        bail!(
            "criteria in {} must include at least one shell command",
            path.display()
        );
    }

    if path.file_stem().and_then(|value| value.to_str()) != Some(criteria.goal_id.as_str()) {
        bail!(
            "criteria filename {} must match goal_id {}",
            path.display(),
            criteria.goal_id
        );
    }

    Ok(criteria)
}

const ALLOWED_REVIEWER_COMMANDS: &[&str] = &["git", "grep", "test", "diff", "wc", "cat", "echo", "ls", "find", "cargo", "true", "false", "exit"];

fn run_shell_bounded(
    paths: &PraxisPaths,
    command: &str,
    max_output_bytes: usize,
) -> Result<Option<String>> {
    let cmd_prefix = command.split_whitespace().next().unwrap_or("");
    let cmd_basename = cmd_prefix.split('/').next_back().unwrap_or("");
    if !ALLOWED_REVIEWER_COMMANDS.contains(&cmd_basename) {
        anyhow::bail!("reviewer command rejected (not in allowlist): `{command}`");
    }
    let output = Command::new("/bin/sh")
        .arg("-lc")
        .arg(command)
        .current_dir(&paths.data_dir)
        .output()
        .with_context(|| format!("failed to execute reviewer command `{command}`"))?;

    if output.status.success() {
        return Ok(None);
    }

    // Cap captured output to the budget ceiling.
    let combined = {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let raw = format!("{stdout}\n{stderr}");
        if raw.len() > max_output_bytes {
            format!(
                "{}… (truncated to {max_output_bytes} bytes)",
                &raw[..max_output_bytes]
            )
        } else {
            raw
        }
    };

    let detail = combined
        .lines()
        .rev()
        .find(|line| !line.trim().is_empty())
        .unwrap_or("command exited without output")
        .trim()
        .chars()
        .take(200)
        .collect::<String>();

    Ok(Some(format!("`{command}` failed: {detail}")))
}
