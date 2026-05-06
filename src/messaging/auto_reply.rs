//! Auto-reply — proactive messaging via inbound polling + auto-response.
//!
//! #8 Auto-reply (from GAP_ANALYSIS_HERMES_OPENCLAW.md).
//!
//! Enables the agent to automatically respond to inbound messages
//! from configured platforms. Works with the Platform trait and
//! the daemon's inbound polling loop.
//!
//! Architecture:
//! - `AutoReplyConfig` controls per-platform auto-reply behavior
//! - `AutoReplyEngine` processes inbound messages and generates responses
//! - Rate-limited: max N auto-replies per hour per channel
//! - Cooldown between consecutive auto-replies

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Per-platform auto-reply configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoReplyConfig {
    /// Whether auto-reply is enabled.
    #[serde(default)]
    pub enabled: bool,
    /// Maximum auto-replies per hour per channel.
    #[serde(default = "default_max_per_hour")]
    pub max_per_hour: u32,
    /// Minimum seconds between consecutive auto-replies to the same channel.
    #[serde(default = "default_cooldown_secs")]
    pub cooldown_secs: u64,
    /// Platforms where auto-reply is active (empty = all).
    #[serde(default)]
    pub platforms: Vec<String>,
    /// Channel IDs to exclude from auto-reply.
    #[serde(default)]
    pub exclude_channels: Vec<String>,
    /// Whether to require an explicit @mention for auto-reply.
    #[serde(default)]
    pub require_mention: bool,
    /// Custom greeting prefix (empty = none).
    #[serde(default)]
    pub greeting_prefix: String,
}

fn default_max_per_hour() -> u32 {
    10
}
fn default_cooldown_secs() -> u64 {
    60
}

impl Default for AutoReplyConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            max_per_hour: default_max_per_hour(),
            cooldown_secs: default_cooldown_secs(),
            platforms: Vec::new(),
            exclude_channels: Vec::new(),
            require_mention: false,
            greeting_prefix: String::new(),
        }
    }
}

/// Tracks auto-reply state per channel.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct ChannelState {
    /// Number of auto-replies in the current hour window.
    replies_this_hour: u32,
    /// Start of the current hour window.
    hour_window_start: i64,
    /// Timestamp of the last auto-reply.
    last_reply_at: i64,
}

/// The auto-reply engine.
pub struct AutoReplyEngine {
    config: AutoReplyConfig,
    state: HashMap<String, ChannelState>,
    state_path: PathBuf,
}

impl AutoReplyEngine {
    pub fn new(config: AutoReplyConfig, data_dir: &Path) -> Self {
        let state_path = data_dir.join("auto_reply_state.json");
        let mut engine = Self {
            config,
            state: HashMap::new(),
            state_path,
        };
        if let Err(e) = engine.load_state() {
            log::warn!("auto_reply: failed to load state: {:#}", e);
        }
        engine
    }

    /// Check if an auto-reply is allowed for the given channel.
    pub fn can_reply(&mut self, platform: &str, channel_id: &str, is_mention: bool) -> bool {
        if !self.config.enabled {
            return false;
        }

        // Platform filter
        if !self.config.platforms.is_empty() && !self.config.platforms.iter().any(|p| p == platform)
        {
            return false;
        }

        // Excluded channels
        if self.config.exclude_channels.iter().any(|c| c == channel_id) {
            return false;
        }

        // Mention requirement
        if self.config.require_mention && !is_mention {
            return false;
        }

        // Rate limit check
        let now = now_secs();
        let state = self.state.entry(channel_id.to_string()).or_default();

        // Reset hour window if expired
        if now - state.hour_window_start >= 3600 {
            state.replies_this_hour = 0;
            state.hour_window_start = now;
        }

        if state.replies_this_hour >= self.config.max_per_hour {
            log::debug!("auto_reply: rate limited for channel {}", channel_id);
            return false;
        }

        // Cooldown check
        if now - state.last_reply_at < self.config.cooldown_secs as i64 {
            log::debug!("auto_reply: cooldown active for channel {}", channel_id);
            return false;
        }

        true
    }

    /// Record that an auto-reply was sent.
    pub fn record_reply(&mut self, channel_id: &str) {
        let now = now_secs();
        let state = self.state.entry(channel_id.to_string()).or_default();
        state.replies_this_hour += 1;
        state.last_reply_at = now;

        if let Err(e) = self.save_state() {
            log::warn!("auto_reply: failed to save state: {:#}", e);
        }
    }

    /// Generate a formatted auto-reply message.
    pub fn format_reply(&self, text: &str) -> String {
        if self.config.greeting_prefix.is_empty() {
            text.to_string()
        } else {
            format!("{} {}", self.config.greeting_prefix, text)
        }
    }

    /// Get current config.
    pub fn config(&self) -> &AutoReplyConfig {
        &self.config
    }

    /// Update config at runtime.
    pub fn set_config(&mut self, config: AutoReplyConfig) {
        self.config = config;
    }

    fn load_state(&mut self) -> Result<()> {
        if !self.state_path.exists() {
            return Ok(());
        }
        let raw = fs::read_to_string(&self.state_path)?;
        self.state = serde_json::from_str(&raw)?;
        Ok(())
    }

    fn save_state(&self) -> Result<()> {
        let json = serde_json::to_string_pretty(&self.state)?;
        fs::write(&self.state_path, json)?;
        Ok(())
    }
}

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_by_default() {
        let tmp = std::env::temp_dir().join("praxis_autoreply_test");
        let _ = fs::create_dir_all(&tmp);
        let engine = AutoReplyEngine::new(AutoReplyConfig::default(), &tmp);
        assert!(!engine.config().enabled);
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn rate_limiting() {
        let tmp = std::env::temp_dir().join("praxis_autoreply_test2");
        let _ = fs::create_dir_all(&tmp);
        let config = AutoReplyConfig {
            enabled: true,
            max_per_hour: 2,
            cooldown_secs: 0,
            ..Default::default()
        };
        let mut engine = AutoReplyEngine::new(config, &tmp);
        assert!(engine.can_reply("telegram", "ch1", false));
        engine.record_reply("ch1");
        assert!(engine.can_reply("telegram", "ch1", false));
        engine.record_reply("ch1");
        assert!(!engine.can_reply("telegram", "ch1", false)); // hit limit
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn mention_required() {
        let tmp = std::env::temp_dir().join("praxis_autoreply_test3");
        let _ = fs::create_dir_all(&tmp);
        let config = AutoReplyConfig {
            enabled: true,
            require_mention: true,
            ..Default::default()
        };
        let mut engine = AutoReplyEngine::new(config, &tmp);
        assert!(!engine.can_reply("telegram", "ch1", false));
        assert!(engine.can_reply("telegram", "ch1", true));
        let _ = fs::remove_dir_all(&tmp);
    }
}
