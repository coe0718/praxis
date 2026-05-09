//! Tool Use Policy Engine — per-platform and per-channel tool access control.
//!
//! Provides fine-grained control over which tools are available in which
//! contexts. Supports:
//! - Platform-level presets (Telegram, Discord, Slack, etc.)
//! - Per-channel overrides
//! - Composite toolsets (union/intersection of multiple sets)
//! - Tool-level enable/disable per channel

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// Top-level policy configuration persisted as `tool_policy.toml`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ToolPolicy {
    /// Named toolsets — reusable collections of tool names.
    #[serde(default)]
    pub toolsets: HashMap<String, ToolsetDef>,

    /// Platform-level presets — default toolsets for each messaging platform.
    #[serde(default)]
    pub platforms: HashMap<String, PlatformPolicy>,

    /// Per-channel overrides — channel IDs mapped to channel-specific policies.
    #[serde(default)]
    pub channels: HashMap<String, ChannelPolicy>,

    /// Global default toolset applied when no platform or channel matches.
    #[serde(default)]
    pub default_allow_all: bool,
}

/// A named collection of tool names.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolsetDef {
    /// Tool names in this set.
    pub tools: Vec<String>,
    /// Description of what this toolset is for.
    #[serde(default)]
    pub description: Option<String>,
}

/// Platform-level policy for a messaging platform.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformPolicy {
    /// Toolset to use for this platform (references a named toolset).
    pub toolset: Option<String>,
    /// Individual tool overrides — explicit enable/disable per tool.
    #[serde(default)]
    pub overrides: HashMap<String, bool>,
    /// If true, require the bot to be @mentioned before processing.
    #[serde(default)]
    pub require_mention: bool,
    /// List of user IDs allowed to interact with the bot on this platform.
    #[serde(default)]
    pub allowed_users: Vec<String>,
    /// If true, allow all tools not explicitly disabled.
    #[serde(default)]
    pub allow_all: bool,
}

/// Per-channel policy override.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelPolicy {
    /// Platform this channel belongs to (e.g. "telegram", "discord").
    pub platform: Option<String>,
    /// Toolset override for this channel.
    pub toolset: Option<String>,
    /// Individual tool overrides for this channel.
    #[serde(default)]
    pub overrides: HashMap<String, bool>,
    /// Ephemeral prompt to inject for this channel.
    #[serde(default)]
    pub ephemeral_prompt: Option<String>,
}

impl ToolPolicy {
    /// Load from a TOML file, returning default if absent.
    pub fn load_or_default(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let raw = fs::read_to_string(path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        toml::from_str(&raw).with_context(|| format!("invalid TOML in {}", path.display()))
    }

    /// Resolve the effective set of allowed tools for a given channel + platform.
    ///
    /// Resolution order:
    /// 1. Channel-level overrides (most specific)
    /// 2. Platform-level preset
    /// 3. Global default (allow_all or empty)
    pub fn resolve_tools(&self, channel_id: &str, platform: &str) -> ResolvedToolPolicy {
        let mut allowed: Option<HashSet<String>> = None;
        let mut disabled: HashSet<String> = HashSet::new();

        // Step 1: Platform preset
        if let Some(pp) = self.platforms.get(platform) {
            if pp.allow_all {
                allowed = None; // None = all allowed
            } else if let Some(ts_name) = &pp.toolset
                && let Some(ts) = self.toolsets.get(ts_name)
            {
                allowed = Some(ts.tools.iter().cloned().collect());
            }
            for (tool, enabled) in &pp.overrides {
                if !enabled {
                    disabled.insert(tool.clone());
                }
            }
        }

        // Step 2: Channel overrides
        if let Some(cp) = self.channels.get(channel_id) {
            if let Some(ts_name) = &cp.toolset
                && let Some(ts) = self.toolsets.get(ts_name)
            {
                allowed = Some(ts.tools.iter().cloned().collect());
            }
            for (tool, enabled) in &cp.overrides {
                if !enabled {
                    disabled.insert(tool.clone());
                } else {
                    disabled.remove(tool);
                }
            }
        }

        // Step 3: Global default
        if allowed.is_none() && !self.default_allow_all {
            // No platform or channel specified a toolset — deny everything
            // unless the global default allows all.
            allowed = Some(HashSet::new());
        }

        ResolvedToolPolicy {
            allowed,
            disabled,
            require_mention: self
                .platforms
                .get(platform)
                .map(|p| p.require_mention)
                .unwrap_or(false),
            ephemeral_prompt: self
                .channels
                .get(channel_id)
                .and_then(|c| c.ephemeral_prompt.clone()),
        }
    }
}

/// The resolved effective tool policy for a specific context.
#[derive(Debug, Clone)]
pub struct ResolvedToolPolicy {
    /// If Some, only these tools are allowed. If None, all tools are allowed.
    pub allowed: Option<HashSet<String>>,
    /// These tools are explicitly disabled regardless of the allowed set.
    pub disabled: HashSet<String>,
    /// Whether @mention is required.
    pub require_mention: bool,
    /// Ephemeral prompt for this channel.
    pub ephemeral_prompt: Option<String>,
}

impl ResolvedToolPolicy {
    /// Check if a specific tool is allowed.
    pub fn is_tool_allowed(&self, tool_name: &str) -> bool {
        if self.disabled.contains(tool_name) {
            return false;
        }
        match &self.allowed {
            Some(set) => set.contains(tool_name),
            None => true,
        }
    }
}
