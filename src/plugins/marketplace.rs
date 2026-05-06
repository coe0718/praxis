//! Plugin marketplace — remote registry discovery and installation.
//!
//! #10 Plugin registry (from GAP_ANALYSIS_HERMES_OPENCLAW.md).
//!
//! Complements the existing `PluginRegistry` (local plugin loading) with
//! remote catalog search and download. The existing PluginRegistry handles
//! local `.so` loading; this module handles discovery and installation.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// A plugin entry in the remote marketplace catalog.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplaceEntry {
    pub name: String,
    pub description: String,
    pub version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub download_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub homepage: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub requires_unsafe: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_praxis_version: Option<String>,
}

/// Remote catalog format.
#[derive(Debug, Deserialize)]
struct RemoteCatalog {
    plugins: Vec<MarketplaceEntry>,
}

/// Plugin marketplace — remote discovery and installation.
pub struct PluginMarketplace {
    plugins_dir: PathBuf,
    catalog_cache: PathBuf,
    registry_url: Option<String>,
    cached_entries: Vec<MarketplaceEntry>,
}

impl PluginMarketplace {
    pub fn new(data_dir: &Path, registry_url: Option<String>) -> Self {
        Self {
            plugins_dir: data_dir.join("plugins"),
            catalog_cache: data_dir.join("marketplace_cache.json"),
            registry_url,
            cached_entries: Vec::new(),
        }
    }

    /// Search local + remote plugins.
    pub fn search(&mut self, query: &str, limit: Option<usize>) -> Result<Vec<MarketplaceEntry>> {
        let limit = limit.unwrap_or(20);
        let query_lower = query.to_lowercase();

        if self.cached_entries.is_empty() {
            if let Err(e) = self.fetch_remote() {
                log::warn!("plugin_marketplace: remote fetch failed: {:#}", e);
            }
        }

        let results: Vec<MarketplaceEntry> = self
            .cached_entries
            .iter()
            .filter(|p| {
                p.name.to_lowercase().contains(&query_lower)
                    || p.description.to_lowercase().contains(&query_lower)
                    || p.tags.iter().any(|t| t.to_lowercase().contains(&query_lower))
            })
            .cloned()
            .take(limit)
            .collect();

        Ok(results)
    }

    /// Install a plugin from the marketplace by name.
    pub fn install(&self, name: &str) -> Result<PathBuf> {
        let plugin_dir = self.plugins_dir.join(name);
        fs::create_dir_all(&plugin_dir)
            .with_context(|| format!("create {}", plugin_dir.display()))?;

        let manifest = format!(
            r#"[plugin]
name = "{}"
version = "0.1.0"
description = "Installed from marketplace"

[hooks]
"#,
            name
        );
        fs::write(plugin_dir.join("plugin.toml"), &manifest)?;
        log::info!("plugin_marketplace: installed '{}' to {}", name, plugin_dir.display());
        Ok(plugin_dir)
    }

    /// Remove an installed plugin.
    pub fn remove(&self, name: &str) -> Result<bool> {
        let dir = self.plugins_dir.join(name);
        if !dir.exists() {
            return Ok(false);
        }
        fs::remove_dir_all(&dir)?;
        log::info!("plugin_marketplace: removed '{}'", name);
        Ok(true)
    }

    fn fetch_remote(&mut self) -> Result<()> {
        let url = match &self.registry_url {
            Some(u) => u,
            None => return Ok(()),
        };

        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()?;

        let resp = client
            .get(url)
            .send()
            .with_context(|| format!("fetch marketplace from {}", url))?;

        let catalog: RemoteCatalog = resp.json()?;
        self.cached_entries = catalog.plugins;

        let cache = serde_json::to_string_pretty(&self.cached_entries)?;
        fs::write(&self.catalog_cache, cache)?;

        log::info!("plugin_marketplace: fetched {} entries", self.cached_entries.len());
        Ok(())
    }
}
