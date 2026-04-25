use std::{fs, path::Path};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use crate::config::AppConfig;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProfileSettings {
    pub profiles: Vec<ExecutionProfile>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExecutionProfile {
    pub name: String,
    #[serde(default)]
    pub backend_override: Option<String>,
    #[serde(default)]
    pub local_first_fallback: Option<bool>,
    #[serde(default)]
    pub context_ceiling_pct: Option<f32>,
    /// Override the context window token budget for this profile.
    #[serde(default)]
    pub context_window_tokens: Option<usize>,
    /// Cap the number of tool calls per session (useful for lite/low-power installs).
    #[serde(default)]
    pub max_session_tool_calls: Option<usize>,
    /// Prevent sub-agent spawning entirely when true.
    #[serde(default)]
    pub disable_sub_agents: Option<bool>,
}

impl Default for ProfileSettings {
    fn default() -> Self {
        Self {
            profiles: vec![
                profile("quality", None, None, None),
                profile("budget", Some("router"), Some(true), Some(0.65)),
                profile("offline", Some("ollama"), Some(true), Some(0.55)),
                profile("deterministic", Some("stub"), Some(false), Some(0.30)),
                ExecutionProfile {
                    name: "lite".to_string(),
                    backend_override: None,
                    local_first_fallback: Some(false),
                    context_ceiling_pct: Some(0.50),
                    context_window_tokens: Some(6_000),
                    max_session_tool_calls: Some(5),
                    disable_sub_agents: Some(true),
                },
            ],
        }
    }
}

impl ProfileSettings {
    pub fn load_or_default(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let raw = fs::read_to_string(path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let settings: Self =
            toml::from_str(&raw).with_context(|| format!("invalid TOML in {}", path.display()))?;
        settings.validate()?;
        Ok(settings)
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
        let raw = toml::to_string_pretty(self).context("failed to serialize profile settings")?;
        fs::write(path, raw).with_context(|| format!("failed to write {}", path.display()))
    }

    pub fn apply(&self, config: &AppConfig) -> Result<AppConfig> {
        let profile = self
            .profiles
            .iter()
            .find(|profile| profile.name == config.agent.profile)
            .with_context(|| format!("unknown model profile {}", config.agent.profile))?;
        let mut adjusted = config.clone();
        if let Some(backend) = &profile.backend_override {
            adjusted.agent.backend = backend.clone();
        }
        if let Some(local_first) = profile.local_first_fallback {
            adjusted.agent.local_first_fallback = local_first;
        }
        if let Some(ceiling) = profile.context_ceiling_pct {
            adjusted.agent.context_ceiling_pct = ceiling;
        }
        if let Some(tokens) = profile.context_window_tokens {
            adjusted.context.window_tokens = tokens;
        }
        if let Some(max_calls) = profile.max_session_tool_calls {
            adjusted.agent.max_session_tool_calls = Some(max_calls);
        }
        if let Some(disable) = profile.disable_sub_agents {
            adjusted.agent.disable_sub_agents = disable;
        }
        adjusted.validate()?;
        Ok(adjusted)
    }

    pub fn validate(&self) -> Result<()> {
        if self.profiles.is_empty() {
            bail!("profile settings must include at least one profile");
        }
        let mut names = std::collections::BTreeSet::new();
        for profile in &self.profiles {
            if profile.name.trim().is_empty() {
                bail!("profile names must not be empty");
            }
            if !names.insert(profile.name.clone()) {
                bail!("duplicate model profile {}", profile.name);
            }
            if let Some(backend) = &profile.backend_override
                && !matches!(backend.as_str(), "stub" | "claude" | "openai" | "ollama" | "router")
            {
                bail!("unsupported backend override {}", backend);
            }
            if let Some(ceiling) = profile.context_ceiling_pct
                && (!(0.0..=1.0).contains(&ceiling) || ceiling == 0.0)
            {
                bail!("profile {} has invalid context ceiling", profile.name);
            }
        }
        Ok(())
    }
}

fn profile(
    name: &str,
    backend_override: Option<&str>,
    local_first_fallback: Option<bool>,
    context_ceiling_pct: Option<f32>,
) -> ExecutionProfile {
    ExecutionProfile {
        name: name.to_string(),
        backend_override: backend_override.map(ToString::to_string),
        local_first_fallback,
        context_ceiling_pct,
        context_window_tokens: None,
        max_session_tool_calls: None,
        disable_sub_agents: None,
    }
}

#[cfg(test)]
mod tests {
    use crate::config::AppConfig;

    use super::ProfileSettings;

    #[test]
    fn budget_profile_reweights_backend_and_context() {
        let mut config = AppConfig::default_for_data_dir("/tmp/praxis".into());
        config.agent.profile = "budget".to_string();

        let applied = ProfileSettings::default().apply(&config).unwrap();
        assert_eq!(applied.agent.backend, "router");
        assert!(applied.agent.local_first_fallback);
        assert_eq!(applied.agent.context_ceiling_pct, 0.65);
    }
}
