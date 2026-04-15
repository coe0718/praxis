//! Synthetic example generation — produces `(context, action, outcome)` triples
//! from completed sessions and appends them to `evals/examples.jsonl`.
//!
//! Examples are lightweight training signals that can later be used for:
//! - Few-shot prompting during Orient/Act phases.
//! - Offline fine-tuning datasets.
//! - Regression tests against known-good trajectories.
//!
//! ## Format (`evals/examples.jsonl`)
//!
//! Each line is a JSON object:
//! ```json
//! {
//!   "id": "20260101T120000Z-review-success",
//!   "generated_at": "2026-01-01T12:00:00Z",
//!   "context": "Goal: fix CI flakiness. Tool calls: read_file, write_file. Memory hits: 3.",
//!   "action": "Identified race condition in test setup; patched parallel teardown.",
//!   "outcome": "success",
//!   "goal_id": "goal-123",
//!   "session_id": 42,
//!   "quality_score": 0.9
//! }
//! ```

use std::{
    fs::{self, OpenOptions},
    io::Write as IoWrite,
    path::Path,
};

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::paths::PraxisPaths;

/// Maximum number of example records retained in the JSONL file.
const MAX_EXAMPLES: usize = 500;

/// A single synthetic `(context, action, outcome)` training example.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyntheticExample {
    /// Unique identifier — `<timestamp>-<slug>`.
    pub id: String,
    pub generated_at: DateTime<Utc>,
    /// Natural-language description of the situation the agent was in.
    pub context: String,
    /// What the agent did — key decision or tool sequence.
    pub action: String,
    /// Session outcome label (`"success"`, `"partial"`, `"failure"`, etc.).
    pub outcome: String,
    /// Source session ID from the database.
    pub session_id: Option<i64>,
    /// Source goal ID string.
    pub goal_id: Option<String>,
    /// Normalised quality score [0.0, 1.0] from the QualityStore, if available.
    pub quality_score: Option<f64>,
}

impl SyntheticExample {
    /// Construct a new example, generating a stable `id` from the timestamp and outcome.
    pub fn new(
        context: impl Into<String>,
        action: impl Into<String>,
        outcome: impl Into<String>,
    ) -> Self {
        let now = Utc::now();
        let outcome_str = outcome.into();
        let slug: String = outcome_str
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '-')
            .take(20)
            .collect();
        let id = format!("{}-{}", now.format("%Y%m%dT%H%M%SZ"), slug);
        Self {
            id,
            generated_at: now,
            context: context.into(),
            action: action.into(),
            outcome: outcome_str,
            session_id: None,
            goal_id: None,
            quality_score: None,
        }
    }

    pub fn with_session_id(mut self, id: i64) -> Self {
        self.session_id = Some(id);
        self
    }

    pub fn with_goal_id(mut self, id: impl Into<String>) -> Self {
        self.goal_id = Some(id.into());
        self
    }

    pub fn with_quality_score(mut self, score: f64) -> Self {
        self.quality_score = Some(score.clamp(0.0, 1.0));
        self
    }
}

// ── I/O ───────────────────────────────────────────────────────────────────────

/// Append a single example to the JSONL log, pruning if over the retention cap.
pub fn record_example(path: &Path, example: &SyntheticExample) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    let line = serde_json::to_string(example).context("failed to serialise synthetic example")?;
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .with_context(|| format!("failed to open {}", path.display()))?;
    writeln!(file, "{line}").with_context(|| format!("failed to write {}", path.display()))?;
    prune_examples(path)
}

/// Load the last `limit` examples from the JSONL log.
pub fn load_recent_examples(path: &Path, limit: usize) -> Result<Vec<SyntheticExample>> {
    let raw = match fs::read_to_string(path) {
        Ok(r) => r,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e).with_context(|| format!("failed to read {}", path.display())),
    };
    let examples: Vec<SyntheticExample> = raw
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect();
    Ok(examples.into_iter().rev().take(limit).collect())
}

/// Return the path to the examples file given the project paths.
pub fn examples_file(paths: &PraxisPaths) -> std::path::PathBuf {
    paths.evals_dir.join("examples.jsonl")
}

fn prune_examples(path: &Path) -> Result<()> {
    let raw = fs::read_to_string(path).unwrap_or_default();
    let lines: Vec<&str> = raw.lines().filter(|l| !l.trim().is_empty()).collect();
    if lines.len() <= MAX_EXAMPLES {
        return Ok(());
    }
    let kept = &lines[lines.len() - MAX_EXAMPLES..];
    let mut content = kept.join("\n");
    content.push('\n');
    fs::write(path, content).with_context(|| format!("failed to prune {}", path.display()))
}

// ── Generation helpers ────────────────────────────────────────────────────────

/// Build a context string from the fields available at Reflect time.
///
/// Callers should supplement with more specific information when available.
pub fn build_context(
    goal_title: Option<&str>,
    action_summary: &str,
    memory_hits: usize,
    tool_calls: usize,
) -> String {
    let mut parts = Vec::new();
    if let Some(goal) = goal_title {
        parts.push(format!("Goal: {goal}"));
    }
    if !action_summary.is_empty() {
        parts.push(format!("Action summary: {action_summary}"));
    }
    if memory_hits > 0 {
        parts.push(format!("Memory hits: {memory_hits}"));
    }
    if tool_calls > 0 {
        parts.push(format!("Tool calls: {tool_calls}"));
    }
    if parts.is_empty() {
        "No context available.".to_string()
    } else {
        parts.join(". ")
    }
}

/// Only generate examples for outcomes that carry useful signal.
pub fn is_useful_outcome(outcome: &str) -> bool {
    !outcome.trim().is_empty() && outcome != "idle" && outcome != "skipped"
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    #[test]
    fn record_and_load_roundtrip() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("examples.jsonl");

        let ex = SyntheticExample::new("context text", "did something", "success")
            .with_session_id(7)
            .with_quality_score(0.85);

        record_example(&path, &ex).unwrap();

        let loaded = load_recent_examples(&path, 10).unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].outcome, "success");
        assert_eq!(loaded[0].session_id, Some(7));
    }

    #[test]
    fn build_context_formats_correctly() {
        let ctx = build_context(Some("Fix CI"), "ran tests", 3, 5);
        assert!(ctx.contains("Fix CI"));
        assert!(ctx.contains("Memory hits: 3"));
        assert!(ctx.contains("Tool calls: 5"));
    }

    #[test]
    fn idle_outcome_not_useful() {
        assert!(!is_useful_outcome("idle"));
        assert!(is_useful_outcome("success"));
    }
}
