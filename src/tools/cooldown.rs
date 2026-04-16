//! Granular approval cooldown policies.
//!
//! An operator can declare per-tool (optionally per-path) windows during which
//! a re-approval is not required.  For example: "trust `praxis-data-write` to
//! `JOURNAL.md` for 30 minutes after each approval" so the agent can log
//! without a fresh approval on every line.
//!
//! ## Configuration (in `praxis.toml`)
//!
//! ```toml
//! [[tool_cooldown]]
//! tool_name    = "praxis-data-write"
//! path_pattern = "JOURNAL.md"   # optional glob; omit to match all paths
//! window_secs  = 1800           # 30 minutes
//! ```
//!
//! Runtime state (last-approved timestamps per slot) is persisted in
//! `tool_cooldowns.json`.

use std::{collections::HashMap, fs, path::Path};

use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

/// One configured cooldown rule.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CooldownPolicy {
    /// Tool name this rule applies to.
    pub tool_name: String,
    /// Optional path pattern.  When `None`, the rule applies regardless of path.
    #[serde(default)]
    pub path_pattern: Option<String>,
    /// How many seconds after an approval before re-approval is required.
    pub window_secs: u64,
}

impl CooldownPolicy {
    /// True if this policy covers the given tool + path combination.
    pub fn matches(&self, tool_name: &str, path: &str) -> bool {
        if self.tool_name != tool_name {
            return false;
        }
        match &self.path_pattern {
            None => true,
            Some(pattern) => glob_match(pattern, path),
        }
    }
}

/// The slot key used in the cooldown store.
fn slot_key(tool_name: &str, path: &str) -> String {
    format!("{tool_name}::{path}")
}

/// Persisted map of slot → last-approved timestamp.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CooldownStore {
    #[serde(default)]
    last_approved: HashMap<String, DateTime<Utc>>,
}

impl CooldownStore {
    pub fn load(path: &Path) -> Result<Self> {
        match fs::read_to_string(path) {
            Ok(raw) => serde_json::from_str(&raw)
                .with_context(|| format!("invalid cooldown store at {}", path.display())),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(e).with_context(|| format!("failed to read {}", path.display())),
        }
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        let raw =
            serde_json::to_string_pretty(self).context("failed to serialize cooldown store")?;
        fs::write(path, raw).with_context(|| format!("failed to write {}", path.display()))
    }

    /// Record an approval for the given tool + path at `now`.
    pub fn record_approval(&mut self, tool_name: &str, path: &str, now: DateTime<Utc>) {
        self.last_approved.insert(slot_key(tool_name, path), now);
    }

    /// True if the last approval for this slot is still within the policy window.
    pub fn is_within_cooldown(
        &self,
        policies: &[CooldownPolicy],
        tool_name: &str,
        path: &str,
        now: DateTime<Utc>,
    ) -> bool {
        let Some(policy) = policies.iter().find(|p| p.matches(tool_name, path)) else {
            return false;
        };
        let key = slot_key(tool_name, path);
        let Some(&last_at) = self.last_approved.get(&key) else {
            return false;
        };
        now - last_at < Duration::seconds(policy.window_secs as i64)
    }

    /// Purge entries older than the maximum policy window to keep the file trim.
    pub fn prune(&mut self, max_window_secs: u64, now: DateTime<Utc>) {
        let cutoff = now - Duration::seconds(max_window_secs as i64);
        self.last_approved.retain(|_, ts| *ts > cutoff);
    }
}

/// Minimal glob: supports `*` as a wildcard matching any sequence of characters.
fn glob_match(pattern: &str, value: &str) -> bool {
    if !pattern.contains('*') {
        return pattern == value;
    }
    let parts: Vec<&str> = pattern.split('*').collect();
    let mut remaining = value;
    for (i, part) in parts.iter().enumerate() {
        if i == 0 {
            if !remaining.starts_with(part) {
                return false;
            }
            remaining = &remaining[part.len()..];
        } else if i == parts.len() - 1 {
            return remaining.ends_with(part);
        } else {
            match remaining.find(part) {
                Some(pos) => remaining = &remaining[pos + part.len()..],
                None => return false,
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::glob_match;

    #[test]
    fn glob_exact_match() {
        assert!(glob_match("JOURNAL.md", "JOURNAL.md"));
        assert!(!glob_match("JOURNAL.md", "GOALS.md"));
    }

    #[test]
    fn glob_wildcard() {
        assert!(glob_match("*.md", "JOURNAL.md"));
        assert!(glob_match("JOURNAL*", "JOURNAL.md"));
        assert!(!glob_match("*.rs", "JOURNAL.md"));
    }
}
