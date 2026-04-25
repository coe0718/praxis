use std::{fs, path::Path};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use super::ProviderAttempt;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UsageBudgetPolicy {
    pub run: UsageBudgetRule,
    pub ask: UsageBudgetRule,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UsageBudgetRule {
    pub max_attempts: usize,
    pub max_tokens: i64,
    pub max_cost_usd: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsageBudgetMode {
    Run,
    Ask,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UsageBudgetDecision {
    pub blocked: bool,
    pub summary: String,
}

impl Default for UsageBudgetPolicy {
    fn default() -> Self {
        Self {
            run: UsageBudgetRule {
                max_attempts: 6,
                max_tokens: 3_000,
                max_cost_usd: 0.25,
            },
            ask: UsageBudgetRule {
                max_attempts: 1,
                max_tokens: 600,
                max_cost_usd: 0.05,
            },
        }
    }
}

impl UsageBudgetPolicy {
    pub fn load_or_default(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let raw = fs::read_to_string(path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let policy: Self =
            toml::from_str(&raw).with_context(|| format!("invalid TOML in {}", path.display()))?;
        policy.validate()?;
        Ok(policy)
    }

    pub fn save_if_missing(&self, path: &Path) -> Result<()> {
        if path.exists() {
            return Ok(());
        }
        self.validate()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        let raw =
            toml::to_string_pretty(self).context("failed to serialize usage budget policy")?;
        fs::write(path, raw).with_context(|| format!("failed to write {}", path.display()))
    }

    pub fn validate(&self) -> Result<()> {
        self.run.validate("run")?;
        self.ask.validate("ask")
    }

    pub fn rule(&self, mode: UsageBudgetMode) -> &UsageBudgetRule {
        match mode {
            UsageBudgetMode::Run => &self.run,
            UsageBudgetMode::Ask => &self.ask,
        }
    }
}

impl UsageBudgetRule {
    pub fn validate(&self, label: &str) -> Result<()> {
        if self.max_attempts == 0 {
            bail!("{label}.max_attempts must be greater than 0");
        }
        if self.max_tokens <= 0 {
            bail!("{label}.max_tokens must be greater than 0");
        }
        if self.max_cost_usd <= 0.0 {
            bail!("{label}.max_cost_usd must be greater than 0");
        }
        Ok(())
    }

    pub fn check_attempts(
        &self,
        attempts: &[ProviderAttempt],
        mode: UsageBudgetMode,
    ) -> UsageBudgetDecision {
        let attempt_count = attempts.len();
        let tokens_used = attempts.iter().map(ProviderAttempt::tokens_used).sum::<i64>();
        let cost_micros = attempts.iter().map(|attempt| attempt.estimated_cost_micros).sum::<i64>();
        let blocked = attempt_count >= self.max_attempts
            || tokens_used >= self.max_tokens
            || cost_micros >= usd_to_micros(self.max_cost_usd);

        UsageBudgetDecision {
            blocked,
            summary: format!(
                "{} budget {}: attempts={}/{} tokens={}/{} est_cost_usd={:.6}/{:.6}",
                mode.label(),
                if blocked { "blocked" } else { "ok" },
                attempt_count,
                self.max_attempts,
                tokens_used,
                self.max_tokens,
                micros_to_usd(cost_micros),
                self.max_cost_usd
            ),
        }
    }

    pub fn check_estimate(
        &self,
        estimated_tokens: i64,
        mode: UsageBudgetMode,
    ) -> UsageBudgetDecision {
        let blocked = 1 > self.max_attempts
            || estimated_tokens >= self.max_tokens
            || 0.0 >= self.max_cost_usd;
        UsageBudgetDecision {
            blocked,
            summary: format!(
                "{} budget {}: estimated_tokens={}/{} max_attempts={} max_cost_usd={:.6}",
                mode.label(),
                if blocked { "blocked" } else { "ok" },
                estimated_tokens,
                self.max_tokens,
                self.max_attempts,
                self.max_cost_usd
            ),
        }
    }
}

impl UsageBudgetMode {
    fn label(self) -> &'static str {
        match self {
            Self::Run => "run",
            Self::Ask => "ask",
        }
    }
}

pub fn estimate_tokens(input: &str) -> i64 {
    input.split_whitespace().map(str::len).sum::<usize>().div_ceil(4) as i64
}

fn usd_to_micros(value: f64) -> i64 {
    (value * 1_000_000.0).round() as i64
}

fn micros_to_usd(value: i64) -> f64 {
    value as f64 / 1_000_000.0
}
