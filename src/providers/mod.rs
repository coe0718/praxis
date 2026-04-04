use std::{fs, path::Path};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProviderSettings {
    pub providers: Vec<ProviderRoute>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProviderRoute {
    pub provider: String,
    pub model: String,
    #[serde(default)]
    pub base_url: Option<String>,
    #[serde(default)]
    pub input_cost_per_million_usd: Option<f64>,
    #[serde(default)]
    pub output_cost_per_million_usd: Option<f64>,
}

impl Default for ProviderSettings {
    fn default() -> Self {
        Self {
            providers: vec![
                ProviderRoute::new("claude", "claude-3-5-sonnet-latest", None),
                ProviderRoute::new("openai", "gpt-5.4-mini", None),
                ProviderRoute::new("ollama", "llama3.2", Some("http://127.0.0.1:11434")),
            ],
        }
    }
}

impl ProviderSettings {
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
        let raw = toml::to_string_pretty(self).context("failed to serialize provider settings")?;
        fs::write(path, raw).with_context(|| format!("failed to write {}", path.display()))
    }

    pub fn validate(&self) -> Result<()> {
        if self.providers.is_empty() {
            bail!("providers configuration must include at least one route");
        }
        for route in &self.providers {
            route.validate()?;
        }
        Ok(())
    }

    pub fn first_for(&self, provider: &str) -> Option<ProviderRoute> {
        self.providers
            .iter()
            .find(|route| route.provider == provider)
            .cloned()
    }
}

impl ProviderRoute {
    fn new(provider: &str, model: &str, base_url: Option<&str>) -> Self {
        Self {
            provider: provider.to_string(),
            model: model.to_string(),
            base_url: base_url.map(ToString::to_string),
            input_cost_per_million_usd: None,
            output_cost_per_million_usd: None,
        }
    }

    pub fn validate(&self) -> Result<()> {
        if !matches!(self.provider.as_str(), "claude" | "openai" | "ollama") {
            bail!("unsupported provider {}", self.provider);
        }
        if self.model.trim().is_empty() {
            bail!("provider {} must define a model", self.provider);
        }
        Ok(())
    }

    pub fn estimated_cost_micros(&self, input_tokens: i64, output_tokens: i64) -> i64 {
        let input = self
            .input_cost_per_million_usd
            .map(|price| (input_tokens as f64 / 1_000_000.0) * price * 1_000_000.0)
            .unwrap_or(0.0);
        let output = self
            .output_cost_per_million_usd
            .map(|price| (output_tokens as f64 / 1_000_000.0) * price * 1_000_000.0)
            .unwrap_or(0.0);
        (input + output).round() as i64
    }
}
