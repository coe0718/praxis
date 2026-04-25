//! Lite mode — reduces sub-agent usage, tightens budgets, and simplifies
//! behaviour for Raspberry Pi or low-power installs.
//!
//! When `praxis.toml` contains `[agent] lite = true`:
//! - context ceiling drops to 60% (from 80%)
//! - no speculative execution branches
//! - no sub-agent reviewers (self-review only, logged)
//! - no synthetic example generation
//! - no daily learning runs
//! - no morning brief generation
//! - reduced anatomy refresh frequency
//! - disabled dashboard SSE stream
//!
//! Lite mode is about staying functional on constrained hardware, not about
//! cutting safety.  Approval queues, loop guards, and file-mutation circuit
//! breakers all remain active.

use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

/// Lite-mode configuration persisted in `praxis.toml`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LiteMode {
    pub enabled: bool,
    #[serde(default = "default_context_ceiling_pct")]
    pub context_ceiling_pct: f32,
    #[serde(default = "default_disable_speculative")]
    pub disable_speculative: bool,
    #[serde(default = "default_disable_subagent_reviewers")]
    pub disable_subagent_reviewers: bool,
    #[serde(default = "default_disable_synthetic_examples")]
    pub disable_synthetic_examples: bool,
    #[serde(default = "default_disable_learning")]
    pub disable_learning: bool,
    #[serde(default = "default_disable_brief")]
    pub disable_brief: bool,
    #[serde(default = "default_anatomy_refresh_hours")]
    pub anatomy_refresh_hours: u64,
    #[serde(default = "default_disable_sse")]
    pub disable_sse: bool,
    #[serde(default = "default_disable_deterministic")]
    pub disable_deterministic: bool,
}

impl Default for LiteMode {
    fn default() -> Self {
        Self {
            enabled: false,
            context_ceiling_pct: default_context_ceiling_pct(),
            disable_speculative: default_disable_speculative(),
            disable_subagent_reviewers: default_disable_subagent_reviewers(),
            disable_synthetic_examples: default_disable_synthetic_examples(),
            disable_learning: default_disable_learning(),
            disable_brief: default_disable_brief(),
            anatomy_refresh_hours: default_anatomy_refresh_hours(),
            disable_sse: default_disable_sse(),
        }
    }
}

fn default_context_ceiling_pct() -> f32 { 0.60 }
fn default_disable_speculative() -> bool { true }
fn default_disable_subagent_reviewers() -> bool { true }
fn default_disable_synthetic_examples() -> bool { true }
fn default_disable_learning() -> bool { true }
fn default_disable_brief() -> bool { true }
fn default_anatomy_refresh_hours() -> u64 { 48 }
fn default_disable_sse() -> bool { true }

impl LiteMode {
    /// Load lite mode settings from a `praxis.toml` file.
    /// Returns defaults when the file is missing or has no `[agent]` section.
    pub fn from_file(path: &Path) -> anyhow::Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let raw = fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("failed to read {}: {e}", path.display()))?;
        let config: toml::Value = toml::from_str(&raw)
            .map_err(|e| anyhow::anyhow!("invalid TOML in {}: {e}", path.display()))?;
        let agent = config.get("agent").and_then(|v| v.as_table());
        let enabled = agent
            .and_then(|t| t.get("lite"))
            .and_then(toml::Value::as_bool)
            .unwrap_or(false);

        let mut lite = Self::default();
        lite.enabled = enabled;
        if let Some(t) = agent {
            if let Some(v) = t.get("lite_context_ceiling_pct").and_then(toml::Value::as_float) {
                lite.context_ceiling_pct = v as f32;
            }
            if let Some(v) = t.get("lite_disable_speculative").and_then(toml::Value::as_bool) {
                lite.disable_speculative = v;
            }
            if let Some(v) = t.get("lite_disable_subagent_reviewers").and_then(toml::Value::as_bool) {
                lite.disable_subagent_reviewers = v;
            }
            if let Some(v) = t.get("lite_disable_synthetic_examples").and_then(toml::Value::as_bool) {
                lite.disable_synthetic_examples = v;
            }
            if let Some(v) = t.get("lite_disable_learning").and_then(toml::Value::as_bool) {
                lite.disable_learning = v;
            }
            if let Some(v) = t.get("lite_disable_brief").and_then(toml::Value::as_bool) {
                lite.disable_brief = v;
            }
            if let Some(v) = t.get("lite_anatomy_refresh_hours").and_then(toml::Value::as_integer) {
                lite.anatomy_refresh_hours = v as u64;
            }
            if let Some(v) = t.get("lite_disable_sse").and_then(toml::Value::as_bool) {
                lite.disable_sse = v;
            }
        }
        Ok(lite)
    }

    /// Returns `true` if a capability should be skipped in lite mode.
    pub fn skip_capability(&self, cap: LiteCapability) -> bool {
        if !self.enabled {
            return false;
        }
        match cap {
            LiteCapability::Speculative => self.disable_speculative,
            LiteCapability::SubagentReviewer => self.disable_subagent_reviewers,
            LiteCapability::SyntheticExamples => self.disable_synthetic_examples,
            LiteCapability::Learning => self.disable_learning,
            LiteCapability::Brief => self.disable_brief,
            LiteCapability::SseStream => self.disable_sse,
            LiteCapability::Deterministic => self.disable_deterministic,
        }
    }
}

/// Named capabilities that lite mode can gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiteCapability {
    Speculative,
    SubagentReviewer,
    SyntheticExamples,
    Learning,
    Brief,
    SseStream,
    Deterministic,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn defaults_when_file_missing() {
        let dir = tempdir().unwrap();
        let lite = LiteMode::from_file(&dir.path().join("praxis.toml")).unwrap();
        assert!(!lite.enabled);
        assert!(!lite.skip_capability(LiteCapability::Speculative));
    }

    #[test]
    fn enabled_via_toml() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("praxis.toml");
        fs::write(&path, "[agent]\nlite = true\n").unwrap();
        let lite = LiteMode::from_file(&path).unwrap();
        assert!(lite.enabled);
        assert!(lite.skip_capability(LiteCapability::Learning));
        assert!(lite.skip_capability(LiteCapability::Speculative));
    }

    #[test]
    fn disabled_skips_nothing() {
        let lite = LiteMode::default();
        assert!(!lite.skip_capability(LiteCapability::Speculative));
    }
}
