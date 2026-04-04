use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::config::{AppConfig, ContextSourceConfig};

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
struct AdaptiveState {
    #[serde(default)]
    updated_at: Option<String>,
    #[serde(default)]
    sources: BTreeMap<String, SourceStats>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
struct SourceStats {
    #[serde(default)]
    successful_sessions: u32,
    #[serde(default)]
    unsuccessful_sessions: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OutcomeScore {
    Success,
    Failure,
    Ignore,
}

pub(crate) fn adapt_config(config: &AppConfig, path: &Path) -> Result<AppConfig> {
    let mut adjusted = config.clone();
    adjusted.context.budget = AdaptiveState::load(path)?.apply(&config.context.budget);
    Ok(adjusted)
}

pub(crate) fn record_context_feedback(
    path: &Path,
    ended_at: DateTime<Utc>,
    sources: &[String],
    outcome: &str,
) -> Result<()> {
    let score = classify_outcome(outcome);
    if score == OutcomeScore::Ignore || sources.is_empty() {
        return Ok(());
    }

    let mut state = AdaptiveState::load(path)?;
    state.updated_at = Some(ended_at.to_rfc3339());
    for source in unique_sources(sources) {
        let stats = state.sources.entry(source).or_default();
        if score == OutcomeScore::Success {
            stats.successful_sessions += 1;
        } else {
            stats.unsuccessful_sessions += 1;
        }
    }
    state.save(path)
}

impl AdaptiveState {
    fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }

        let raw = fs::read_to_string(path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        serde_json::from_str(&raw)
            .with_context(|| format!("invalid adaptive context state in {}", path.display()))
    }

    fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        let raw = serde_json::to_string_pretty(self)
            .context("failed to serialize adaptive context state")?;
        fs::write(path, raw).with_context(|| format!("failed to write {}", path.display()))
    }

    fn apply(&self, budget: &[ContextSourceConfig]) -> Vec<ContextSourceConfig> {
        let base_total = budget
            .iter()
            .map(|entry| entry.max_pct)
            .sum::<f32>()
            .max(f32::EPSILON);
        let mut adjusted = budget
            .iter()
            .cloned()
            .map(|mut entry| {
                entry.max_pct = (entry.max_pct * self.multiplier(&entry.source)).max(0.01);
                entry
            })
            .collect::<Vec<_>>();
        let adjusted_total = adjusted
            .iter()
            .map(|entry| entry.max_pct)
            .sum::<f32>()
            .max(f32::EPSILON);
        let scale = base_total / adjusted_total;
        for entry in &mut adjusted {
            entry.max_pct *= scale;
        }
        adjusted
    }

    fn multiplier(&self, source: &str) -> f32 {
        let Some(stats) = self.sources.get(source) else {
            return 1.0;
        };
        let total = stats.successful_sessions + stats.unsuccessful_sessions;
        if total == 0 {
            return 1.0;
        }

        let success_rate = (stats.successful_sessions as f32 + 1.0) / (total as f32 + 2.0);
        (1.0 + ((success_rate - 0.5) * 0.4)).clamp(0.8, 1.2)
    }
}

fn unique_sources(sources: &[String]) -> BTreeSet<String> {
    sources.iter().cloned().collect()
}

fn classify_outcome(outcome: &str) -> OutcomeScore {
    match outcome {
        "review_failed" | "eval_failed" | "blocked_loop_guard" | "budget_exhausted" => {
            OutcomeScore::Failure
        }
        "deferred_quiet_hours" => OutcomeScore::Ignore,
        _ => OutcomeScore::Success,
    }
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use crate::config::AppConfig;

    use super::{AdaptiveState, adapt_config, record_context_feedback};

    #[test]
    fn preserves_total_budget_while_biasing_successful_sources() {
        let temp = tempdir().unwrap();
        let state_path = temp.path().join("adaptive.json");
        record_context_feedback(
            &state_path,
            chrono::Utc::now(),
            &[String::from("task"), String::from("task")],
            "task_selected",
        )
        .unwrap();
        let config = AppConfig::default_for_data_dir("/tmp/praxis".into());
        let adjusted = adapt_config(&config, &state_path).unwrap();
        let original_task = config
            .context
            .budget
            .iter()
            .find(|entry| entry.source == "task")
            .unwrap()
            .max_pct;
        let new_task = adjusted
            .context
            .budget
            .iter()
            .find(|entry| entry.source == "task")
            .unwrap()
            .max_pct;

        assert!(new_task > original_task);
        assert!(
            (config.context.budget.iter().map(|e| e.max_pct).sum::<f32>()
                - adjusted
                    .context
                    .budget
                    .iter()
                    .map(|e| e.max_pct)
                    .sum::<f32>())
            .abs()
                < 0.0001
        );
    }

    #[test]
    fn ignores_deferred_sessions_and_records_failures() {
        let temp = tempdir().unwrap();
        let state_path = temp.path().join("adaptive.json");
        record_context_feedback(
            &state_path,
            chrono::Utc::now(),
            &[String::from("journal")],
            "deferred_quiet_hours",
        )
        .unwrap();
        assert_eq!(
            AdaptiveState::load(&state_path).unwrap(),
            AdaptiveState::default()
        );

        record_context_feedback(
            &state_path,
            chrono::Utc::now(),
            &[String::from("journal")],
            "review_failed",
        )
        .unwrap();
        let stored = AdaptiveState::load(&state_path).unwrap();
        assert_eq!(stored.sources["journal"].unsuccessful_sessions, 1);
    }
}
