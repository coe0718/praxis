//! Plugin System — dynamic plugin loading and lifecycle management.
//!
//! #3 Plugin System + #5 Plugin Surface
//!
//! Plugins are TOML manifests + optional Rust `.so` dynamic libraries.
//! They can:
//!   - Register new tools
//!   - Register new messaging platforms
//!   - Hook into lifecycle events (pre_prompt, post_response, on_error)
//!   - Transform tool results before returning to agent
//!   - Block tool execution entirely
//!   - Register slash commands
//!
//! Plugin manifest (`plugin.toml` in plugin dir):
//! ```toml
//! name = "my-plugin"
//! version = "0.1.0"
//! author = "you"
//!
//! [hooks]
//! pre_prompt = "scripts/pre_prompt.sh"
//! post_response = "scripts/post_response.sh"
//! tool_block = ["rm -rf /", "format filesystem"]
//!
//! [tools]
//! my_tool = "scripts/my_tool.sh"
//!
//! [[messaging_platforms]]
//! name = "teams"
//! enabled = true
//! ```

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use crate::paths::PraxisPaths;

/// Loaded plugin with all its capabilities.
#[derive(Debug, Clone)]
pub struct Plugin {
    pub name: String,
    pub version: String,
    pub hooks: PluginHooks,
    pub tools: Vec<ToolRegistration>,
    pub platforms: Vec<PlatformRegistration>,
}

impl Plugin {
    pub fn from_dir(dir: &Path) -> Result<Self> {
        let manifest_path = dir.join("plugin.toml");
        if !manifest_path.exists() {
            bail!("plugin manifest not found: {}", manifest_path.display());
        }
        let raw = fs::read_to_string(&manifest_path)?;
        let manifest: PluginManifest = toml::from_str(&raw)
            .with_context(|| format!("invalid plugin.toml in {}", dir.display()))?;
        let name = manifest.name.clone();
        manifest.try_into().with_context(|| format!("plugin {name}"))
    }
}

#[derive(Debug, Deserialize)]
struct PluginManifest {
    name: String,
    version: String,
    #[serde(default)]
    hooks: PluginHooks,
    #[serde(default)]
    tools: Vec<ToolRegistration>,
    #[serde(default)]
    platforms: Vec<PlatformRegistration>,
}

impl TryFrom<PluginManifest> for Plugin {
    type Error = anyhow::Error;
    fn try_from(m: PluginManifest) -> std::result::Result<Self, Self::Error> {
        Ok(Self {
            name: m.name,
            version: m.version,
            hooks: m.hooks,
            tools: m.tools,
            platforms: m.platforms,
        })
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PluginHooks {
    #[serde(default)]
    pub pre_prompt: Option<PathBuf>,
    #[serde(default)]
    pub post_response: Option<PathBuf>,
    #[serde(default)]
    pub on_error: Option<PathBuf>,
    /// Run a shell command; if non-zero exit, block the tool.
    #[serde(default)]
    pub tool_block: Vec<String>,
    /// Commands that, if present in tool output, rewrite/transform the output.
    #[serde(default)]
    pub tool_rewrite: Vec<String>,
}

impl Default for PluginHooks {
    fn default() -> Self {
        Self {
            pre_prompt: None,
            post_response: None,
            on_error: None,
            tool_block: vec![],
            tool_rewrite: vec![],
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ToolRegistration {
    pub name: String,
    pub command: PathBuf,
    #[serde(default)]
    pub required_level: u8,
    #[serde(default)]
    pub requires_approval: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PlatformRegistration {
    pub name: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
}
fn default_true() -> bool {
    true
}

/// Global plugin registry.
pub struct PluginRegistry {
    plugins: HashMap<String, Plugin>,
    paths: PraxisPaths,
}

impl PluginRegistry {
    pub fn new(paths: &PraxisPaths) -> Self {
        Self {
            plugins: HashMap::new(),
            paths: paths.clone(),
        }
    }

    /// Scan plugins directory and load all plugins.
    pub fn load_all(&mut self) -> Result<()> {
        let plugins_dir = self.paths.data_dir.join("plugins");
        if !plugins_dir.exists() {
            fs::create_dir_all(&plugins_dir)?;
            return Ok(());
        }
        for entry in fs::read_dir(&plugins_dir)? {
            let entry = entry?;
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            match Plugin::from_dir(&path) {
                Ok(plugin) => {
                    let name = plugin.name.clone();
                    self.plugins.insert(name.clone(), plugin);
                    log::info!("loaded plugin: {}", name);
                }
                Err(e) => {
                    log::warn!("failed to load plugin at {}: {}", path.display(), e);
                }
            }
        }
        Ok(())
    }

    /// Get a plugin by name.
    pub fn get(&self, name: &str) -> Option<&Plugin> {
        self.plugins.get(name)
    }

    /// List all loaded plugins.
    pub fn list(&self) -> Vec<&str> {
        self.plugins.keys().map(|s| s.as_str()).collect()
    }

    /// Check if a shell command should be blocked by any plugin.
    pub fn should_block(&self, command: &str) -> Option<String> {
        for plugin in self.plugins.values() {
            for pattern in &plugin.hooks.tool_block {
                if command.contains(pattern.as_str()) {
                    return Some(format!("blocked by plugin: {}", plugin.name));
                }
            }
        }
        None
    }

    /// Collect all tool registrations from all plugins.
    pub fn tool_registrations(&self) -> Vec<(String, String)> {
        let mut out = Vec::new();
        for plugin in self.plugins.values() {
            for tool in &plugin.tools {
                out.push((tool.name.clone(), plugin.name.clone()));
            }
        }
        out
    }
}
