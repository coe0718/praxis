use std::{fs, path::Path};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

/// Which wire protocol to use when talking to this provider.
/// Inferred from `provider` name if not explicitly set.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "kebab-case")]
pub enum ProviderProtocol {
    #[default]
    Anthropic,
    OpenAiCompat,
    Ollama,
}

/// Routing class for rule-based selection.
/// Routes without a class match any class request.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum RouteClass {
    /// Cheap, low-latency models — Orient, ask, lightweight review.
    Fast,
    /// Capable, higher-quality models — Decide, Act, deep review.
    Reliable,
    /// Local / on-device models — privacy-sensitive or offline work.
    Local,
}

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
    /// Explicit wire protocol override. Inferred from provider name when absent.
    #[serde(default)]
    pub protocol: Option<ProviderProtocol>,
    /// Routing class for rule-based selection.
    #[serde(default)]
    pub class: Option<RouteClass>,
    #[serde(default)]
    pub input_cost_per_million_usd: Option<f64>,
    #[serde(default)]
    pub output_cost_per_million_usd: Option<f64>,
    /// Static traffic weight (0.0 = disabled, 1.0 = full). Overridden at runtime
    /// by canary automation via `canary_weights.json`. Defaults to 1.0 when absent.
    #[serde(default)]
    pub weight: Option<f64>,
}

impl Default for ProviderSettings {
    fn default() -> Self {
        Self {
            providers: vec![
                ProviderRoute::new("claude", "claude-3-5-sonnet-latest", None),
                ProviderRoute::new("openai", "gpt-4o-mini", None),
                ProviderRoute::new("ollama", "llama3.2", Some("http://127.0.0.1:11434")),
            ],
        }
    }
}

impl ProviderSettings {
    pub fn load_or_default(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::env_enriched(Self::default()));
        }

        let raw = fs::read_to_string(path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let settings: Self =
            toml::from_str(&raw).with_context(|| format!("invalid TOML in {}", path.display()))?;
        settings.validate()?;
        Ok(Self::env_enriched(settings))
    }

    /// Fill in any provider gaps from standard environment variables.
    /// If the env var for a provider is set but no route for it exists, add a
    /// default route.  Existing configured routes are never overwritten.
    fn env_enriched(mut settings: Self) -> Self {
        let has_claude = settings.providers.iter().any(|r| r.provider == "claude");
        let has_openai = settings.providers.iter().any(|r| r.provider == "openai");
        let has_ollama = settings.providers.iter().any(|r| r.provider == "ollama");

        if !has_claude && std::env::var("ANTHROPIC_API_KEY").is_ok() {
            settings.providers.push(ProviderRoute::new(
                "claude",
                "claude-3-5-sonnet-latest",
                None,
            ));
        }

        if !has_openai && std::env::var("OPENAI_API_KEY").is_ok() {
            let base = std::env::var("OPENAI_BASE_URL").ok();
            settings
                .providers
                .push(ProviderRoute::new("openai", "gpt-4o-mini", base.as_deref()));
        }

        if !has_ollama {
            if let Ok(host) = std::env::var("OLLAMA_HOST") {
                settings
                    .providers
                    .push(ProviderRoute::new("ollama", "llama3.2", Some(&host)));
            }
        }

        settings
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

    /// Return the first route matching the given class, or fall back to the
    /// first unclassed route, or the very first route if nothing else matches.
    pub fn first_for_class(&self, class: &RouteClass) -> Option<ProviderRoute> {
        self.providers
            .iter()
            .find(|r| r.class.as_ref() == Some(class))
            .or_else(|| self.providers.iter().find(|r| r.class.is_none()))
            .cloned()
    }
}

impl ProviderRoute {
    fn new(provider: &str, model: &str, base_url: Option<&str>) -> Self {
        Self {
            provider: provider.to_string(),
            model: model.to_string(),
            base_url: base_url.map(ToString::to_string),
            protocol: None,
            class: None,
            input_cost_per_million_usd: None,
            output_cost_per_million_usd: None,
            weight: None,
        }
    }

    /// Validate the route.  Well-known provider names are accepted as-is.
    /// Any other name is accepted when `base_url` is explicitly set, making the
    /// route an OpenAI-compatible endpoint under a custom name.
    pub fn validate(&self) -> Result<()> {
        if self.model.trim().is_empty() {
            bail!("provider {} must define a model", self.provider);
        }
        match self.provider.as_str() {
            "claude" | "openai" | "ollama" => {}
            custom if self.base_url.is_some() => {
                // Custom provider names are valid when a base_url is set.
                // They are dispatched through the OpenAI-compatible adapter.
                let _ = custom;
            }
            other => bail!(
                "unsupported provider '{other}': set base_url to use it as an OpenAI-compatible endpoint"
            ),
        }
        Ok(())
    }

    /// Resolve the wire protocol for this route.
    pub fn resolved_protocol(&self) -> ProviderProtocol {
        if let Some(proto) = &self.protocol {
            return proto.clone();
        }
        match self.provider.as_str() {
            "claude" => ProviderProtocol::Anthropic,
            "ollama" => ProviderProtocol::Ollama,
            _ => ProviderProtocol::OpenAiCompat,
        }
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
