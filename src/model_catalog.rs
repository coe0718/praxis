//! Model catalog — discovery and listing of available LLM providers/models.
//!
//! Provides a `praxis model list` command and an endpoint for the agent
//! to discover available models at runtime.
use anyhow::{Context, Result};

/// A discovered model available for use.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub provider: String,
    pub family: String,
    pub context_window: Option<u32>,
    pub max_output_tokens: Option<u32>,
    pub supports_function_calling: bool,
    pub supports_vision: bool,
    pub pricing_input_per_1m: Option<f64>,
    pub pricing_output_per_1m: Option<f64>,
}

/// Model source — where the model info comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ModelSource {
    /// From the configured provider routes in provider_routes.rs.
    Configured,
    /// From an online provider catalog API.
    Online,
    /// From a local model file or local server.
    Local,
}

impl std::fmt::Display for ModelSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Configured => write!(f, "configured"),
            Self::Online => write!(f, "online"),
            Self::Local => write!(f, "local"),
        }
    }
}

/// A catalog entry pairing model info with its source.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CatalogEntry {
    pub model: ModelInfo,
    pub source: ModelSource,
}

/// Discover models from configured providers and optional online catalogs.
pub struct ModelCatalog {
    entries: Vec<CatalogEntry>,
}

impl ModelCatalog {
    /// Create an empty model catalog.
    pub fn new() -> Self {
        Self { entries: Vec::new() }
    }

    /// Load models from the provider configuration file (alias for from_config).
    pub fn load_from_config(providers_path: &std::path::Path) -> Result<Self> {
        Self::from_config(providers_path)
    }

    /// Load and parse models from the provider configuration file.
    pub fn from_config(providers_path: &std::path::Path) -> Result<Self> {
        use std::fs;

        let mut catalog = Self::new();

        if !providers_path.exists() {
            log::warn!("model catalog: providers file not found at {}", providers_path.display());
            return Ok(catalog);
        }

        let content =
            fs::read_to_string(providers_path).context("failed to read providers file")?;

        #[derive(serde::Deserialize)]
        struct ProviderEntry {
            name: String,
            provider: String,
            models: Option<Vec<String>>,
        }

        let providers: Vec<ProviderEntry> =
            serde_yaml::from_str(&content).context("failed to parse providers file")?;

        for p in providers {
            let models = p.models.unwrap_or_else(|| {
                vec![
                    format!("{}:default", p.name),
                    format!("{}:fast", p.name),
                    format!("{}:smart", p.name),
                ]
            });

            for model_id in models {
                catalog.entries.push(CatalogEntry {
                    model: ModelInfo {
                        id: model_id.clone(),
                        provider: p.provider.clone(),
                        family: p.name.clone(),
                        context_window: None,
                        max_output_tokens: None,
                        supports_function_calling: true,
                        supports_vision: model_id.contains("vision") || model_id.contains("gpt-4"),
                        pricing_input_per_1m: None,
                        pricing_output_per_1m: None,
                    },
                    source: ModelSource::Configured,
                });
            }
        }

        Ok(catalog)
    }

    /// Add a model to the catalog.
    pub fn add(&mut self, model: ModelInfo, source: ModelSource) {
        self.entries.push(CatalogEntry { model, source });
    }

    /// List all models in the catalog, optionally filtered by provider.
    pub fn list(&self, provider_filter: Option<&str>) -> Vec<&ModelInfo> {
        self.entries
            .iter()
            .filter(|entry| provider_filter.map(|pf| entry.model.provider == pf).unwrap_or(true))
            .map(|entry| &entry.model)
            .collect()
    }

    /// Returns a reference to all catalog entries.
    pub fn entries(&self) -> &[CatalogEntry] {
        &self.entries
    }

    /// Find a model by ID.
    pub fn find(&self, model_id: &str) -> Option<&ModelInfo> {
        self.entries
            .iter()
            .find(|entry| entry.model.id == model_id)
            .map(|entry| &entry.model)
    }

    /// Generate a human-readable table of available models.
    pub fn format_table(&self) -> String {
        if self.entries.is_empty() {
            return "No models found in catalog.".to_string();
        }

        let mut lines = Vec::new();
        lines.push(format!(
            "{:<35} {:<12} {:<10} {:<8} {:<8}",
            "MODEL", "PROVIDER", "FAMILY", "FCALL", "VISION"
        ));
        lines.push("-".repeat(85));

        for entry in &self.entries {
            let m = &entry.model;
            lines.push(format!(
                "{:<35} {:<12} {:<10} {:<8} {:<8}",
                m.id,
                m.provider,
                m.family,
                if m.supports_function_calling { "yes" } else { "no" },
                if m.supports_vision { "yes" } else { "no" },
            ));
        }

        lines.join("\n")
    }

    /// Total number of models in the catalog.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// True if the catalog is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Handle the `praxis model list` command.
pub fn handle_model_list(providers_path: &std::path::Path) -> anyhow::Result<String> {
    let catalog = ModelCatalog::from_config(providers_path)?;

    if catalog.is_empty() {
        return Ok(format!(
            "No models found. Check your providers file at {}.",
            providers_path.display()
        ));
    }

    Ok(format!(
        "Model catalog ({} model(s) from {} source(s)):\n\n{}",
        catalog.len(),
        catalog
            .entries
            .iter()
            .map(|e| e.source.to_string())
            .collect::<std::collections::HashSet<_>>()
            .len(),
        catalog.format_table()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_catalog_empty() {
        let catalog = ModelCatalog::new();
        assert!(catalog.is_empty());
        assert_eq!(catalog.len(), 0);
        assert!(catalog.list(None).is_empty());
    }

    #[test]
    fn model_catalog_add_and_find() {
        let mut catalog = ModelCatalog::new();
        let model = ModelInfo {
            id: "test-model".to_string(),
            provider: "test-provider".to_string(),
            family: "test".to_string(),
            context_window: Some(4096),
            max_output_tokens: Some(1024),
            supports_function_calling: true,
            supports_vision: false,
            pricing_input_per_1m: None,
            pricing_output_per_1m: None,
        };

        catalog.add(model.clone(), ModelSource::Local);
        assert!(!catalog.is_empty());
        assert_eq!(catalog.len(), 1);

        let found = catalog.find("test-model");
        assert!(found.is_some());
        assert_eq!(found.unwrap().provider, "test-provider");

        let not_found = catalog.find("nonexistent");
        assert!(not_found.is_none());
    }

    #[test]
    fn model_catalog_filter_by_provider() {
        let mut catalog = ModelCatalog::new();
        catalog.add(
            ModelInfo {
                id: "model-a".to_string(),
                provider: "openai".to_string(),
                family: "gpt".to_string(),
                context_window: None,
                max_output_tokens: None,
                supports_function_calling: true,
                supports_vision: false,
                pricing_input_per_1m: None,
                pricing_output_per_1m: None,
            },
            ModelSource::Configured,
        );
        catalog.add(
            ModelInfo {
                id: "model-b".to_string(),
                provider: "anthropic".to_string(),
                family: "claude".to_string(),
                context_window: None,
                max_output_tokens: None,
                supports_function_calling: true,
                supports_vision: false,
                pricing_input_per_1m: None,
                pricing_output_per_1m: None,
            },
            ModelSource::Configured,
        );

        let all = catalog.list(None);
        assert_eq!(all.len(), 2);

        let openai = catalog.list(Some("openai"));
        assert_eq!(openai.len(), 1);
        assert_eq!(openai[0].provider, "openai");
    }
}
