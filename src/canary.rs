use std::{collections::BTreeSet, fs, path::Path};

use anyhow::{Context, Result, bail};
use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::{
    backend::{AgentBackend, ConfiguredBackend},
    config::AppConfig,
    paths::PraxisPaths,
    providers::ProviderSettings,
    quality::{EvalTrigger, LocalEvalSuite},
    storage::{EvalSeverity, EvalStatus},
};

const CANARY_PROMPT: &str = "Reply with exactly: PraxisCanaryReady";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModelCanaryLedger {
    pub records: Vec<ModelCanaryRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModelCanaryRecord {
    pub provider: String,
    pub model: String,
    pub status: CanaryStatus,
    pub checked_at: String,
    pub summary: String,
    pub eval_failures: usize,
    /// Running count of consecutive passes; reset to 0 on any failure.
    #[serde(default)]
    pub consecutive_passes: u32,
}

/// Routes to freeze after sustained failures; persisted as `canary_frozen.json`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CanaryFreezeState {
    /// Set of `"provider/model"` strings that are currently frozen.
    #[serde(default)]
    pub frozen: std::collections::BTreeSet<String>,
}

impl CanaryFreezeState {
    pub fn load_or_default(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let raw = fs::read_to_string(path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        serde_json::from_str(&raw).with_context(|| format!("invalid JSON in {}", path.display()))
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        let raw = serde_json::to_string_pretty(self).context("failed to serialize freeze state")?;
        fs::write(path, raw).with_context(|| format!("failed to write {}", path.display()))
    }

    pub fn key(provider: &str, model: &str) -> String {
        format!("{provider}/{model}")
    }

    pub fn is_frozen(&self, provider: &str, model: &str) -> bool {
        self.frozen.contains(&Self::key(provider, model))
    }

    pub fn freeze(&mut self, provider: &str, model: &str) {
        self.frozen.insert(Self::key(provider, model));
    }

    pub fn unfreeze(&mut self, provider: &str, model: &str) {
        self.frozen.remove(&Self::key(provider, model));
    }
}

/// Actions taken by canary automation after a run.
#[derive(Debug, Default)]
pub struct CanaryAutomation {
    /// Routes promoted (unfrozen) after `PROMOTION_THRESHOLD` consecutive passes.
    pub promoted: Vec<String>,
    /// Routes frozen after failing (triggers rollback).
    pub rolled_back: Vec<String>,
}

/// Consecutive passes required before a previously-frozen route is promoted.
const PROMOTION_THRESHOLD: u32 = 3;

/// Apply promotion and rollback automation based on the latest canary ledger.
///
/// - Routes that just failed (consecutive_passes reset to 0 from >0) are frozen.
/// - Routes that reached `PROMOTION_THRESHOLD` consecutive passes are unfrozen.
pub fn apply_automation(
    ledger: &ModelCanaryLedger,
    freeze_path: &Path,
    prev_ledger: &ModelCanaryLedger,
) -> Result<CanaryAutomation> {
    let mut freeze = CanaryFreezeState::load_or_default(freeze_path)?;
    let mut automation = CanaryAutomation::default();

    for record in &ledger.records {
        let key = CanaryFreezeState::key(&record.provider, &record.model);

        // Find previous consecutive_passes for this route.
        let prev_passes = prev_ledger
            .records
            .iter()
            .find(|r| r.provider == record.provider && r.model == record.model)
            .map(|r| r.consecutive_passes)
            .unwrap_or(0);

        match record.status {
            CanaryStatus::Passed => {
                if record.consecutive_passes >= PROMOTION_THRESHOLD
                    && freeze.is_frozen(&record.provider, &record.model)
                {
                    freeze.unfreeze(&record.provider, &record.model);
                    automation.promoted.push(key);
                }
            }
            CanaryStatus::Failed => {
                // Rollback if it was passing before (consecutive_passes just reset to 0).
                if prev_passes > 0 && !freeze.is_frozen(&record.provider, &record.model) {
                    freeze.freeze(&record.provider, &record.model);
                    automation.rolled_back.push(key);
                }
            }
        }
    }

    if !automation.promoted.is_empty() || !automation.rolled_back.is_empty() {
        freeze.save(freeze_path)?;
    }

    Ok(automation)
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CanaryStatus {
    Passed,
    Failed,
}

impl ModelCanaryLedger {
    pub fn load_or_default(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self {
                records: Vec::new(),
            });
        }
        let raw = fs::read_to_string(path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let ledger: Self = serde_json::from_str(&raw)
            .with_context(|| format!("invalid JSON in {}", path.display()))?;
        ledger.validate()?;
        Ok(ledger)
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        self.validate()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        let raw =
            serde_json::to_string_pretty(self).context("failed to serialize canary ledger")?;
        fs::write(path, raw).with_context(|| format!("failed to write {}", path.display()))
    }

    pub fn save_if_missing(&self, path: &Path) -> Result<()> {
        if path.exists() {
            return Ok(());
        }
        self.save(path)
    }

    pub fn validate(&self) -> Result<()> {
        let mut keys = BTreeSet::new();
        for record in &self.records {
            if record.provider.trim().is_empty() || record.model.trim().is_empty() {
                bail!("canary records must include provider and model");
            }
            if !keys.insert((record.provider.clone(), record.model.clone())) {
                bail!(
                    "duplicate canary record for {}/{}",
                    record.provider,
                    record.model
                );
            }
        }
        Ok(())
    }

    pub fn replace(&mut self, mut record: ModelCanaryRecord) {
        // Carry forward the consecutive_passes counter.
        let prev_passes = self
            .records
            .iter()
            .find(|r| r.provider == record.provider && r.model == record.model)
            .map(|r| r.consecutive_passes)
            .unwrap_or(0);
        record.consecutive_passes = match record.status {
            CanaryStatus::Passed => prev_passes + 1,
            CanaryStatus::Failed => 0,
        };
        self.records.retain(|existing| {
            existing.provider != record.provider || existing.model != record.model
        });
        self.records.push(record);
        self.records.sort_by(|left, right| {
            left.provider
                .cmp(&right.provider)
                .then(left.model.cmp(&right.model))
        });
    }

    pub fn passed(&self, provider: &str, model: &str) -> bool {
        self.records.iter().any(|record| {
            record.provider == provider
                && record.model == model
                && record.status == CanaryStatus::Passed
        })
    }
}

pub fn run_canaries(
    config: &AppConfig,
    paths: &PraxisPaths,
    provider: Option<&str>,
) -> Result<Vec<ModelCanaryRecord>> {
    let providers = target_providers(config, paths, provider)?;
    let prev_ledger = ModelCanaryLedger::load_or_default(&paths.model_canary_file)?;
    let mut ledger = prev_ledger.clone();
    let mut records = Vec::new();

    for provider in providers {
        let record = run_provider_canary(config, paths, &provider)?;
        ledger.replace(record.clone());
        records.push(record);
    }

    ledger.save(&paths.model_canary_file)?;

    match apply_automation(&ledger, &paths.canary_freeze_file, &prev_ledger) {
        Ok(automation) => {
            for route in &automation.promoted {
                log::info!("canary: promoted {route} (>={PROMOTION_THRESHOLD} consecutive passes)");
            }
            for route in &automation.rolled_back {
                log::warn!("canary: rolled back {route} (fresh failure after passing streak)");
            }
        }
        Err(e) => log::warn!("canary automation failed: {e}"),
    }

    Ok(records)
}

fn target_providers(
    config: &AppConfig,
    paths: &PraxisPaths,
    provider: Option<&str>,
) -> Result<Vec<String>> {
    if let Some(provider) = provider {
        return Ok(vec![provider.to_string()]);
    }

    if config.agent.backend == "router" {
        let mut unique = BTreeSet::new();
        let mut providers = Vec::new();
        for route in ProviderSettings::load_or_default(&paths.providers_file)?.providers {
            if unique.insert(route.provider.clone()) {
                providers.push(route.provider);
            }
        }
        return Ok(providers);
    }

    Ok(vec![config.agent.backend.clone()])
}

fn run_provider_canary(
    config: &AppConfig,
    paths: &PraxisPaths,
    provider: &str,
) -> Result<ModelCanaryRecord> {
    let route = ProviderSettings::load_or_default(&paths.providers_file)?
        .first_for(provider)
        .with_context(|| format!("no provider route configured for {provider}"))?;

    let mut probe_config = config.clone();
    probe_config.agent.backend = provider.to_string();
    probe_config.agent.model_pin = (config.agent.backend == provider)
        .then(|| config.agent.model_pin.clone())
        .flatten();
    probe_config.agent.freeze_on_model_regression = false;

    let backend = ConfiguredBackend::from_runtime(&probe_config, paths)?;
    let checked_at = Utc::now().to_rfc3339();

    match backend.answer_prompt(CANARY_PROMPT) {
        Ok(output) => {
            let evals = LocalEvalSuite.run_trigger(paths, EvalTrigger::Canary)?;
            let failed = evals
                .iter()
                .filter(|result| result.status == EvalStatus::Failed)
                .count();
            let trust_failed = evals.iter().any(|result| {
                result.status == EvalStatus::Failed
                    && result.severity == EvalSeverity::TrustDamaging
            });
            let status = if failed == 0 {
                CanaryStatus::Passed
            } else {
                CanaryStatus::Failed
            };
            let attempt = output
                .attempts
                .iter()
                .rev()
                .find(|attempt| attempt.success)
                .cloned()
                .with_context(|| {
                    format!("provider {provider} returned no successful canary attempt")
                })?;
            Ok(ModelCanaryRecord {
                provider: attempt.provider,
                model: attempt.model,
                status,
                checked_at,
                summary: format!(
                    "{} canary probe completed; eval_failures={} trust_failures={} response={}",
                    provider,
                    failed,
                    usize::from(trust_failed),
                    output.summary.chars().take(80).collect::<String>()
                ),
                eval_failures: failed,
                consecutive_passes: 0, // will be updated by replace()
            })
        }
        Err(error) => Ok(ModelCanaryRecord {
            provider: route.provider,
            model: route.model,
            status: CanaryStatus::Failed,
            checked_at,
            summary: format!("{} canary probe failed: {}", provider, error),
            eval_failures: 0,
            consecutive_passes: 0, // will be updated by replace()
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::{CanaryStatus, ModelCanaryLedger, ModelCanaryRecord};

    #[test]
    fn replacing_records_keeps_latest_status_per_provider_model() {
        let mut ledger = ModelCanaryLedger {
            records: Vec::new(),
        };
        ledger.replace(ModelCanaryRecord {
            provider: "claude".to_string(),
            model: "sonnet".to_string(),
            status: CanaryStatus::Failed,
            checked_at: "2026-04-04T00:00:00Z".to_string(),
            summary: "failed".to_string(),
            eval_failures: 1,
        });
        ledger.replace(ModelCanaryRecord {
            provider: "claude".to_string(),
            model: "sonnet".to_string(),
            status: CanaryStatus::Passed,
            checked_at: "2026-04-04T01:00:00Z".to_string(),
            summary: "passed".to_string(),
            eval_failures: 0,
        });

        assert_eq!(ledger.records.len(), 1);
        assert!(ledger.passed("claude", "sonnet"));
    }
}
