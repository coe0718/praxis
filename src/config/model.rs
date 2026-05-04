use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    str::FromStr,
};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::time::parse_timezone;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AppConfig {
    pub instance: InstanceConfig,
    pub runtime: RuntimeConfig,
    pub database: DatabaseConfig,
    pub security: SecurityConfig,
    pub agent: AgentConfig,
    #[serde(default)]
    pub context: ContextConfig,
    /// Runtime feature flags — wrap risky capabilities so they can be toggled
    /// without a code change.  All default to `false` (off) so new flags are
    /// opt-in until explicitly enabled in `praxis.toml`.
    #[serde(default)]
    pub features: FeatureFlags,
    /// Per-channel ephemeral prompts — channel/chat IDs mapped to prompt text.
    /// These prompts are injected into the agent's context when processing a
    /// message from the corresponding channel but don't persist across sessions.
    #[serde(default)]
    pub ephemeral_prompts: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InstanceConfig {
    pub name: String,
    pub timezone: String,
    pub data_dir: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RuntimeConfig {
    pub quiet_hours_start: String,
    pub quiet_hours_end: String,
    pub state_file: PathBuf,
    #[serde(default)]
    pub daily_backup_snapshots: bool,
    #[serde(default = "default_snapshot_retention_days")]
    pub snapshot_retention_days: usize,
    /// When `true`, the watchdog update flow tars the entire `data_dir` into
    /// `backups/praxis-data-<timestamp>.tar.gz` before replacing the binary.
    /// Defaults to `true` (opt-out) so updates are always recoverable.
    #[serde(default = "default_true")]
    pub backup_before_update: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DatabaseConfig {
    pub path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SecurityConfig {
    pub level: u8,
    /// Per-tool approval cooldown windows.  When a slot was approved within
    /// the configured `window_secs`, re-approval is skipped automatically.
    #[serde(default)]
    pub tool_cooldowns: Vec<crate::tools::cooldown::CooldownPolicy>,
    /// When `true`, tool output is scanned for common secret patterns (API
    /// keys, tokens, passwords, private keys) and matches are replaced with
    /// `[REDACTED]`.  Opt-in (default `false`) to avoid corrupting patch
    /// diffs or other structured output.
    #[serde(default)]
    pub redact_secrets: bool,
    /// When `true`, the bot only responds to Telegram messages that contain
    /// an @mention entity directed at it.  Private (1:1) chats are exempt.
    /// Opt-in via `[security] require_mention = true` in `praxis.toml`.
    #[serde(default)]
    pub require_mention: bool,
    /// Whitelist of Telegram user IDs (as strings) allowed to interact with
    /// the bot.  When non-empty, messages from users not in the list are
    /// silently skipped.  Opt-in via `[security] allowed_users = [...]`.
    #[serde(default)]
    pub allowed_users: Vec<String>,
    /// Hardline blocklist — commands matching these patterns are **always**
    /// rejected regardless of approval level.  Unrecoverable block; no
    /// override.  Add patterns like `"rm -rf /"`, `"drop table"`,
    /// `":(){ :|:& };:"`.  Checked before any other policy.
    #[serde(default)]
    pub hardline_blocklist: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentConfig {
    pub backend: String,
    pub context_ceiling_pct: f32,
    #[serde(default = "default_profile")]
    pub profile: String,
    pub model_pin: Option<String>,
    #[serde(default)]
    pub local_first_fallback: bool,
    #[serde(default)]
    pub freeze_on_model_regression: bool,
    /// Enable Anthropic prompt caching on the system prompt.
    /// Reduces cost and latency on repeated Orient / Ask calls with large
    /// context windows.  Only applied when talking to the Claude provider.
    #[serde(default)]
    pub prompt_caching: bool,
    /// Cap the number of tool calls the agent may make in a single session.
    /// `None` means unlimited.  Set by the `lite` profile (default: 5).
    #[serde(default)]
    pub max_session_tool_calls: Option<usize>,
    /// When true, the agent may not spawn sub-agents.  Set by the `lite` profile.
    #[serde(default)]
    pub disable_sub_agents: bool,
    /// Time-to-live in seconds for cached prompt content (default 5 min).
    #[serde(default = "default_prompt_cache_ttl_secs")]
    pub prompt_cache_ttl_secs: u64,
    /// Opt-in extended prompt cache (~1 h).  When `true`, `prompt_cache_ttl_secs`
    /// is overridden to 3600 automatically.
    #[serde(default)]
    pub extended_prompt_cache: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContextConfig {
    pub window_tokens: usize,
    pub budget: Vec<ContextSourceConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContextSourceConfig {
    pub source: String,
    pub priority: u8,
    pub max_pct: f32,
}

impl Default for ContextConfig {
    fn default() -> Self {
        Self {
            window_tokens: 12_000,
            budget: vec![
                source("soul", 1, 0.03),
                source("identity", 2, 0.05),
                source("operator_model", 3, 0.03),
                source("agents", 4, 0.04),
                source("active_goals", 5, 0.09),
                source("do_not_repeat", 6, 0.07),
                source("known_bugs", 7, 0.06),
                source("memory_hot", 8, 0.17),
                source("memory_cold", 9, 0.11),
                source("predictions", 10, 0.04),
                source("patterns", 11, 0.04),
                source("journal", 12, 0.05),
                source("tools", 13, 0.04),
                source("task", 14, 0.18),
            ],
        }
    }
}

impl AppConfig {
    pub fn default_for_data_dir(data_dir: PathBuf) -> Self {
        Self {
            instance: InstanceConfig {
                name: "Praxis".to_string(),
                timezone: "UTC".to_string(),
                data_dir,
            },
            runtime: RuntimeConfig {
                quiet_hours_start: "23:00".to_string(),
                quiet_hours_end: "07:00".to_string(),
                state_file: PathBuf::from("session_state.json"),
                daily_backup_snapshots: false,
                snapshot_retention_days: default_snapshot_retention_days(),
                backup_before_update: true,
            },
            database: DatabaseConfig {
                path: PathBuf::from("praxis.db"),
            },
            security: SecurityConfig {
                level: 2,
                tool_cooldowns: Vec::new(),
                redact_secrets: false,
                require_mention: false,
                allowed_users: Vec::new(),
                hardline_blocklist: Vec::new(),
            },
            agent: AgentConfig {
                backend: "stub".to_string(),
                context_ceiling_pct: 0.80,
                profile: default_profile(),
                model_pin: None,
                local_first_fallback: false,
                freeze_on_model_regression: false,
                prompt_caching: false,
                max_session_tool_calls: None,
                disable_sub_agents: false,
                prompt_cache_ttl_secs: default_prompt_cache_ttl_secs(),
                extended_prompt_cache: false,
            },
            context: ContextConfig::default(),
            features: FeatureFlags::default(),
            ephemeral_prompts: HashMap::new(),
        }
    }

    pub fn load(path: &Path) -> Result<Self> {
        let raw = fs::read_to_string(path)
            .with_context(|| format!("failed to read config from {}", path.display()))?;
        let config: Self =
            toml::from_str(&raw).with_context(|| format!("invalid TOML in {}", path.display()))?;
        config.validate()?;
        Ok(config)
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        self.validate()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        let raw = toml::to_string_pretty(self).context("failed to serialize praxis config")?;
        fs::write(path, raw).with_context(|| format!("failed to write {}", path.display()))?;
        Ok(())
    }

    pub fn with_overridden_data_dir(mut self, data_dir: PathBuf) -> Self {
        self.instance.data_dir = data_dir;
        self
    }

    pub fn set_name(&mut self, name: impl Into<String>) {
        self.instance.name = name.into();
    }

    pub fn set_timezone(&mut self, timezone: impl Into<String>) -> Result<()> {
        let timezone = timezone.into();
        parse_timezone(&timezone)?;
        self.instance.timezone = timezone;
        Ok(())
    }
}

impl FromStr for AppConfig {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        let config: Self = toml::from_str(s).context("invalid Praxis config")?;
        config.validate()?;
        Ok(config)
    }
}

fn source(name: &str, priority: u8, max_pct: f32) -> ContextSourceConfig {
    ContextSourceConfig {
        source: name.to_string(),
        priority,
        max_pct,
    }
}

fn default_profile() -> String {
    "quality".to_string()
}

fn default_true() -> bool {
    true
}

fn default_snapshot_retention_days() -> usize {
    7
}

fn default_prompt_cache_ttl_secs() -> u64 {
    300
}

/// Runtime feature flags for toggling capabilities without code changes.
///
/// Add new flags here as `#[serde(default)] bool` fields.  Every flag must
/// have a corresponding `is_<name>()` accessor that the rest of the crate
/// uses instead of reaching into the struct directly — this makes it easy
/// to add logging or migration logic later.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct FeatureFlags {
    /// Enable credential pooling (multi-key rotation per provider).
    #[serde(default)]
    pub credential_pooling: bool,
    /// Enable the agent-callable cron / scheduled-jobs tool.
    #[serde(default)]
    pub cron_tool: bool,
    /// Enable speculative execution (branching Act phase).
    #[serde(default)]
    pub speculative_execution: bool,
    /// Enable agent-to-agent delegation links in Act phase.
    #[serde(default)]
    pub delegation: bool,
    /// Enable MCP (Model Context Protocol) tool dispatch.
    #[serde(default)]
    pub mcp_dispatch: bool,
}

impl FeatureFlags {
    /// Convenience accessor used by the daemon and session runner.
    pub fn is_enabled(&self, flag: FeatureFlag) -> bool {
        match flag {
            FeatureFlag::CredentialPooling => self.credential_pooling,
            FeatureFlag::CronTool => self.cron_tool,
            FeatureFlag::SpeculativeExecution => self.speculative_execution,
            FeatureFlag::Delegation => self.delegation,
            FeatureFlag::McpDispatch => self.mcp_dispatch,
        }
    }

    /// Return a human-readable list of currently enabled flags (for logging).
    pub fn enabled_list(&self) -> Vec<&'static str> {
        let mut list = Vec::new();
        if self.credential_pooling {
            list.push("credential_pooling");
        }
        if self.cron_tool {
            list.push("cron_tool");
        }
        if self.speculative_execution {
            list.push("speculative_execution");
        }
        if self.delegation {
            list.push("delegation");
        }
        if self.mcp_dispatch {
            list.push("mcp_dispatch");
        }
        list
    }
}

/// Typed enum for checking a specific flag without knowing field names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeatureFlag {
    CredentialPooling,
    CronTool,
    SpeculativeExecution,
    Delegation,
    McpDispatch,
}
