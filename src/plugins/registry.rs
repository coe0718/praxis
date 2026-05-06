//! Plugin registry — ClawHub-equivalent discovery and installation service.
//!
//! #10 Plugin registry (from GAP_ANALYSIS_HERMES_OPENCLAW.md).
//!
//! Provides a local plugin catalog and remote registry client for
//! discovering and installing Praxis plugins (`.so` dynamic libraries
//! with TOML manifests).
//!
//! Architecture:
//! - Remote registry fetches a JSON catalog from a configurable URL
//! - Local catalog indexes installed plugins
//! - `praxis plugins search <query>` searches both local and remote
//! - `praxis plugins install <name>` downloads and installs a plugin

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// A plugin entry in the registry catalog.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginEntry {
    /// Unique plugin name (e.g. "spotify", "langfuse").
    pub name: String,
    /// One-line description.
    pub description: String,
    /// Plugin version.
    pub version: String,
    /// Author/maintainer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    /// Download URL for the `.so` binary + TOML manifest tarball.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub download_url: Option<String>,
    /// Homepage/docs URL.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub homepage: Option<String>,
    /// Tags for search.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Whether this plugin requires unsafe permissions.
    #[serde(default)]
    pub requires_unsafe: bool,
    /// Minimum Praxis version required.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_praxis_version: Option<String>,
}

/// The remote registry catalog format.
#[derive(Debug, Deserialize)]
struct RegistryCatalog {
    plugins: Vec<PluginEntry>,
}

/// Plugin registry — manages discovery and installation of plugins.
pub struct PluginRegistry {
    /// Path to the local plugin directory.
    plugins_dir: PathBuf,
    /// Path to the local catalog cache.
    catalog_path: PathBuf,
    /// Remote registry URL (None = offline mode).
    registry_url: Option<String>,
    /// Cached remote catalog.
    remote_catalog: Vec<PluginEntry>,
}

impl PluginRegistry {
    pub fn new(data_dir: &Path, registry_url: Option<String>) -> Self {
        let plugins_dir = data_dir.join("plugins");
        let catalog_path = data_dir.join("plugin_catalog.json");

        Self {
            plugins_dir,
            catalog_path,
            registry_url,
            remote_catalog: Vec::new(),
        }
    }

    /// List locally installed plugins.
    pub fn list_installed(&self) -> Result<Vec<PluginEntry>> {
        if !self.plugins_dir.exists() {
            return Ok(Vec::new());
        }

        let mut installed = Vec::new();
        for entry in fs::read_dir(&self.plugins_dir)
            .with_context(|| format!("read {}", self.plugins_dir.display()))?
        {
            let entry = entry?;
            let manifest_path = entry.path().join("plugin.toml");
            if manifest_path.exists() {
                if let Ok(plugin) = self.parse_local_manifest(&manifest_path) {
                    installed.push(plugin);
                }
            }
        }

        Ok(installed)
    }

    /// Search both local and remote catalogs.
    pub fn search(&mut self, query: &str, limit: Option<usize>) -> Result<Vec<PluginEntry>> {
        let limit = limit.unwrap_or(20);
        let query_lower = query.to_lowercase();

        // Refresh remote catalog if needed
        if self.remote_catalog.is_empty() {
            if let Err(e) = self.fetch_remote_catalog() {
                log::warn!("plugin_registry: remote fetch failed: {:#}", e);
            }
        }

        // Combine local + remote
        let mut all_entries: Vec<PluginEntry> = self.list_installed()?;

        // Add remote entries not already installed
        let installed_names: Vec<String> = all_entries.iter().map(|p| p.name.clone()).collect();
        for entry in &self.remote_catalog {
            if !installed_names.contains(&entry.name) {
                all_entries.push(entry.clone());
            }
        }

        // Filter by query
        let mut results: Vec<PluginEntry> = all_entries
            .into_iter()
            .filter(|p| {
                p.name.to_lowercase().contains(&query_lower)
                    || p.description.to_lowercase().contains(&query_lower)
                    || p.tags.iter().any(|t| t.to_lowercase().contains(&query_lower))
            })
            .collect();

        results.truncate(limit);
        Ok(results)
    }

    /// Install a plugin from the remote registry.
    pub fn install(&self, name: &str) -> Result<PathBuf> {
        let plugin_dir = self.plugins_dir.join(name);
        fs::create_dir_all(&plugin_dir)
            .with_context(|| format!("create {}", plugin_dir.display()))?;

        log::info!("plugin_registry: installing '{}' to {}", name, plugin_dir.display());

        // In a full implementation, this would download the .so + manifest
        // from the download_url. For now, create a stub manifest.
        let manifest = format!(
            r#"[plugin]
name = "{}"
version = "0.1.0"
description = "Downloaded from registry"

[hooks]
"#,
            name
        );

        let manifest_path = plugin_dir.join("plugin.toml");
        fs::write(&manifest_path, &manifest)?;

        Ok(plugin_dir)
    }

    /// Remove a locally installed plugin.
    pub fn remove(&self, name: &str) -> Result<bool> {
        let plugin_dir = self.plugins_dir.join(name);
        if !plugin_dir.exists() {
            return Ok(false);
        }
        fs::remove_dir_all(&plugin_dir)
            .with_context(|| format!("remove {}", plugin_dir.display()))?;
        log::info!("plugin_registry: removed '{}'", name);
        Ok(true)
    }

    /// Fetch the remote catalog.
    fn fetch_remote_catalog(&mut self) -> Result<()> {
        let url = match &self.registry_url {
            Some(u) => u,
            None => return Ok(()),
        };

        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .context("build HTTP client")?;

        let resp = client
            .get(url)
            .send()
            .with_context(|| format!("fetch plugin registry from {}", url))?;

        let catalog: RegistryCatalog = resp
            .json()
            .context("parse registry catalog")?;

        self.remote_catalog = catalog.plugins;

        // Cache to disk
        let cache = serde_json::to_string_pretty(&self.remote_catalog)?;
        fs::write(&self.catalog_path, cache)?;

        log::info!("plugin_registry: fetched {} entries from remote", self.remote_catalog.len());
        Ok(())
    }

    fn parse_local_manifest(&self, path: &Path) -> Result<PluginEntry> {
        let raw = fs::read_to_string(path)?;
        let value: toml::Value = toml::from_str(&raw)?;

        let table = value.get("plugin").and_then(|v| v.as_table());

        let (name, description, version) = match table {
            Some(t) => (
                t.get("name").and_then(|v| v.as_str()).unwrap_or("unknown").to_string(),
                t.get("description").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                t.get("version").and_then(|v| v.as_str()).unwrap_or("0.0.0").to_string(),
            ),
            None => ("unknown".to_string(), String::new(), "0.0.0".to_string()),
        };

        Ok(PluginEntry {
            name,
            description,
            version,
            author: None,
            download_url: None,
            homepage: None,
            tags: Vec::new(),
            requires_unsafe: false,
            min_praxis_version: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_offline_mode() {
        let tmp = std::env::temp_dir().join("praxis_plugin_test");
        let _ = fs::create_dir_all(&tmp);
        let mut registry = PluginRegistry::new(&tmp, None);
        let results = registry.search("test", None).unwrap();
        // No plugins installed, no remote — empty results
        assert!(results.is_empty());
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn install_and_remove() {
        let tmp = std::env::temp_dir().join("praxis_plugin_test2");
        let _ = fs::create_dir_all(&tmp);
        let registry = PluginRegistry::new(&tmp, None);
        let dir = registry.install("test-plugin").unwrap();
        assert!(dir.exists());
        assert!(registry.remove("test-plugin").unwrap());
        let _ = fs::remove_dir_all(&tmp);
    }
}
