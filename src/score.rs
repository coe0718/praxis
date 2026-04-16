//! Irreplaceability score — a per-session composite metric that quantifies how
//! much the operator would lose if the agent were replaced with a naive
//! assistant.
//!
//! ## Dimensions
//!
//! | Dimension          | What it measures                                        |
//! |--------------------|---------------------------------------------------------|
//! | Anticipation       | Proactive wake hits / total proactive wakes             |
//! | Follow-through     | Goals completed / goals selected                        |
//! | Reliability        | Approvals passed / total tool calls requiring approval  |
//! | Operator dependence| Whether the operator had to intervene (lower is better) |
//!
//! Each dimension is normalised to [0.0, 1.0].  The composite score is a
//! weighted average.  Weights are configurable in the `ScoreWeights` struct;
//! the defaults reflect the design intent (follow-through most important).
//!
//! Scores are appended as JSONL to `score.jsonl` and exposed via
//! `praxis status` and the morning brief.

use std::{
    fs::{self, OpenOptions},
    io::Write as IoWrite,
    path::Path,
};

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Maximum number of score records to retain.
const MAX_RECORDS: usize = 365;

// ── Weights ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreWeights {
    pub anticipation: f64,
    pub follow_through: f64,
    pub reliability: f64,
    pub operator_independence: f64,
}

impl Default for ScoreWeights {
    fn default() -> Self {
        Self {
            anticipation: 0.20,
            follow_through: 0.40,
            reliability: 0.25,
            operator_independence: 0.15,
        }
    }
}

// ── Session score record ──────────────────────────────────────────────────────

/// Raw counts collected during a session — before normalisation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionScoreInput {
    /// Number of proactive wakes that led to meaningful work.
    pub proactive_wake_hits: u32,
    /// Total proactive wakes attempted (denominator for anticipation).
    pub proactive_wakes_total: u32,
    /// Whether the session completed its selected goal.
    pub goal_completed: bool,
    /// Whether a goal was selected at all.
    pub goal_was_selected: bool,
    /// Number of tool calls approved without operator override.
    pub approvals_passed: u32,
    /// Total tool calls that entered the approval queue.
    pub approvals_total: u32,
    /// Whether the operator had to manually intervene (inject task, reject
    /// approval, forcefully stop the session).
    pub operator_intervened: bool,
}

/// A fully computed score for a single session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionScore {
    pub session_id: Option<i64>,
    pub recorded_at: DateTime<Utc>,
    /// [0.0, 1.0] Proactive accuracy.
    pub anticipation: f64,
    /// [0.0, 1.0] Goal completion rate.
    pub follow_through: f64,
    /// [0.0, 1.0] Tool approval pass rate.
    pub reliability: f64,
    /// [0.0, 1.0] Inverse operator intervention rate.
    pub operator_independence: f64,
    /// Weighted composite score — higher is better.
    pub composite: f64,
    pub weights: ScoreWeights,
}

impl SessionScore {
    /// Compute a `SessionScore` from raw input and given weights.
    pub fn compute(input: &SessionScoreInput, weights: &ScoreWeights) -> Self {
        let anticipation = if input.proactive_wakes_total == 0 {
            // No proactive wakes attempted — treat as neutral rather than penalising.
            0.5
        } else {
            input.proactive_wake_hits as f64 / input.proactive_wakes_total as f64
        };

        let follow_through = if !input.goal_was_selected {
            0.5 // No goal = maintenance session; neutral.
        } else if input.goal_completed {
            1.0
        } else {
            0.0
        };

        let reliability = if input.approvals_total == 0 {
            1.0 // No approvals needed — fully autonomous; score max.
        } else {
            input.approvals_passed as f64 / input.approvals_total as f64
        };

        let operator_independence = if input.operator_intervened { 0.0 } else { 1.0 };

        let total_weight = weights.anticipation
            + weights.follow_through
            + weights.reliability
            + weights.operator_independence;

        let composite = if total_weight == 0.0 {
            0.0
        } else {
            (anticipation * weights.anticipation
                + follow_through * weights.follow_through
                + reliability * weights.reliability
                + operator_independence * weights.operator_independence)
                / total_weight
        };

        Self {
            session_id: None,
            recorded_at: Utc::now(),
            anticipation,
            follow_through,
            reliability,
            operator_independence,
            composite,
            weights: weights.clone(),
        }
    }

    pub fn with_session_id(mut self, id: i64) -> Self {
        self.session_id = Some(id);
        self
    }

    /// One-line summary suitable for a status display.
    pub fn summary_line(&self) -> String {
        format!(
            "score {:.2} (anticipation {:.2}, follow-through {:.2}, reliability {:.2}, independence {:.2})",
            self.composite,
            self.anticipation,
            self.follow_through,
            self.reliability,
            self.operator_independence,
        )
    }
}

// ── I/O ───────────────────────────────────────────────────────────────────────

/// Append a score record to the JSONL log.
pub fn record_score(path: &Path, score: &SessionScore) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    let line = serde_json::to_string(score).context("failed to serialise session score")?;
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .with_context(|| format!("failed to open {}", path.display()))?;
    writeln!(file, "{line}").with_context(|| format!("failed to write {}", path.display()))?;
    prune_score_log(path)
}

/// Load the last `limit` score records from the JSONL log.
pub fn load_recent_scores(path: &Path, limit: usize) -> Result<Vec<SessionScore>> {
    let raw = match fs::read_to_string(path) {
        Ok(r) => r,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e).with_context(|| format!("failed to read {}", path.display())),
    };
    let scores: Vec<SessionScore> = raw
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect();
    Ok(scores.into_iter().rev().take(limit).collect())
}

/// Compute the rolling average composite score over the last `limit` sessions.
pub fn rolling_composite(path: &Path, limit: usize) -> Option<f64> {
    let scores = load_recent_scores(path, limit).ok()?;
    if scores.is_empty() {
        return None;
    }
    let sum: f64 = scores.iter().map(|s| s.composite).sum();
    Some(sum / scores.len() as f64)
}

fn prune_score_log(path: &Path) -> Result<()> {
    let raw = fs::read_to_string(path).unwrap_or_default();
    let lines: Vec<&str> = raw.lines().filter(|l| !l.trim().is_empty()).collect();
    if lines.len() <= MAX_RECORDS {
        return Ok(());
    }
    let kept = &lines[lines.len() - MAX_RECORDS..];
    let mut content = kept.join("\n");
    content.push('\n');
    fs::write(path, content).with_context(|| format!("failed to prune {}", path.display()))
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    fn default_input() -> SessionScoreInput {
        SessionScoreInput {
            proactive_wake_hits: 3,
            proactive_wakes_total: 4,
            goal_completed: true,
            goal_was_selected: true,
            approvals_passed: 10,
            approvals_total: 10,
            operator_intervened: false,
        }
    }

    #[test]
    fn perfect_score_is_one() {
        let score = SessionScore::compute(&default_input(), &ScoreWeights::default());
        assert!(score.composite > 0.9, "got {}", score.composite);
    }

    #[test]
    fn intervention_reduces_composite() {
        let mut input = default_input();
        input.operator_intervened = true;
        let score = SessionScore::compute(&input, &ScoreWeights::default());
        assert!(score.operator_independence == 0.0);
        assert!(score.composite < 1.0);
    }

    #[test]
    fn no_goal_selected_is_neutral() {
        let input = SessionScoreInput {
            goal_was_selected: false,
            ..Default::default()
        };
        let score = SessionScore::compute(&input, &ScoreWeights::default());
        assert_eq!(score.follow_through, 0.5);
    }

    #[test]
    fn record_and_load_roundtrip() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("score.jsonl");
        let score =
            SessionScore::compute(&default_input(), &ScoreWeights::default()).with_session_id(1);

        record_score(&path, &score).unwrap();
        let loaded = load_recent_scores(&path, 10).unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].session_id, Some(1));
    }
}
