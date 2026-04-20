//! Per-channel sandboxing — restricts which tools and security levels are
//! accessible to messages arriving from secondary channels (group chats,
//! delegate links, external webhooks).
//!
//! The main operator surface (the primary Telegram chat, the CLI) always has
//! full access.  Every other channel can be assigned a `ChannelSandbox` policy
//! that narrows the permitted tool surface.
//!
//! ## `channel_sandbox.json` format (keyed by channel ID string)
//!
//! ```json
//! {
//!   "-1001234567890": {
//!     "label": "team-chat",
//!     "allowed_tool_kinds": ["internal"],
//!     "denied_tool_name_patterns": ["risky-*"],
//!     "max_security_level": 1,
//!     "force_approval": true
//!   }
//! }
//! ```
//!
//! ## DelegationLink sandboxes
//!
//! When a delegation link carries a `sandbox` field, all inbound task requests
//! arriving on that link are evaluated against the sandbox before the approval
//! queue is consulted.

use std::{collections::HashMap, fs, path::Path};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::{paths::PraxisPaths, tools::ToolKind};

// ── Policy ─────────────────────────────────────────────────────────────────────

/// A named restriction policy for a channel or delegation link.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChannelSandbox {
    /// Human-readable label for display.
    #[serde(default)]
    pub label: String,
    /// Tool kinds permitted in this channel.  Empty = all kinds permitted.
    #[serde(default)]
    pub allowed_tool_kinds: Vec<String>,
    /// Glob patterns of tool names that are always blocked regardless of kind.
    #[serde(default)]
    pub denied_tool_name_patterns: Vec<String>,
    /// Cap on `required_level` — tools with a higher level are blocked.
    /// `None` means no cap (inherits global security level).
    #[serde(default)]
    pub max_security_level: Option<u8>,
    /// When `true`, every tool call requires operator approval even if the
    /// global policy would auto-approve it.
    #[serde(default)]
    pub force_approval: bool,
}

impl ChannelSandbox {
    /// Construct a maximally restrictive sandbox — internal tools only,
    /// security level capped at 1, all tool calls require approval.
    pub fn strict(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            allowed_tool_kinds: vec!["internal".to_string()],
            denied_tool_name_patterns: Vec::new(),
            max_security_level: Some(1),
            force_approval: true,
        }
    }

    /// Construct a read-only sandbox — internal tools, no writes.
    pub fn read_only(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            allowed_tool_kinds: vec!["internal".to_string()],
            denied_tool_name_patterns: vec![
                "praxis-data-write".to_string(),
                "*-write*".to_string(),
            ],
            max_security_level: Some(2),
            force_approval: false,
        }
    }
}

// ── Verdict ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SandboxVerdict {
    /// Permitted — proceed normally.
    Allow,
    /// Blocked by sandbox policy; contains a human-readable reason.
    Block(String),
    /// Permitted but must go through the operator approval queue.
    RequireApproval,
}

// ── Evaluation ────────────────────────────────────────────────────────────────

/// Evaluate a tool against a sandbox policy.
///
/// `tool_name` — the manifest's `name` field.
/// `tool_kind` — the manifest's `kind` field.
/// `required_level` — the manifest's `required_level` field.
pub fn evaluate_tool(
    sandbox: &ChannelSandbox,
    tool_name: &str,
    tool_kind: ToolKind,
    required_level: u8,
) -> SandboxVerdict {
    // Check security level cap.
    if let Some(max) = sandbox.max_security_level
        && required_level > max
    {
        return SandboxVerdict::Block(format!(
            "tool '{tool_name}' requires level {required_level} but sandbox caps at {max}"
        ));
    }

    // Check denied tool name patterns.
    for pattern in &sandbox.denied_tool_name_patterns {
        if glob_match(pattern, tool_name) {
            return SandboxVerdict::Block(format!(
                "tool '{tool_name}' matches denied pattern '{pattern}'"
            ));
        }
    }

    // Check allowed tool kinds.
    if !sandbox.allowed_tool_kinds.is_empty() {
        let kind_str = tool_kind_str(tool_kind);
        if !sandbox
            .allowed_tool_kinds
            .iter()
            .any(|k| k == kind_str || k == "*")
        {
            return SandboxVerdict::Block(format!(
                "tool kind '{kind_str}' is not in sandbox allowed_tool_kinds"
            ));
        }
    }

    // Tool is allowed — check if approval is forced.
    if sandbox.force_approval {
        SandboxVerdict::RequireApproval
    } else {
        SandboxVerdict::Allow
    }
}

fn tool_kind_str(kind: ToolKind) -> &'static str {
    match kind {
        ToolKind::Internal => "internal",
        ToolKind::Shell => "shell",
        ToolKind::Http => "http",
    }
}

// ── Store ──────────────────────────────────────────────────────────────────────

/// Per-channel sandbox policies, keyed by channel ID string.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChannelSandboxStore {
    #[serde(flatten)]
    pub policies: HashMap<String, ChannelSandbox>,
}

impl ChannelSandboxStore {
    pub fn load(path: &Path) -> Result<Self> {
        match fs::read_to_string(path) {
            Ok(raw) => serde_json::from_str(&raw)
                .with_context(|| format!("invalid sandbox store at {}", path.display())),
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
            serde_json::to_string_pretty(self).context("failed to serialise sandbox store")?;
        fs::write(path, raw).with_context(|| format!("failed to write {}", path.display()))
    }

    /// Retrieve the sandbox for a channel, if one is configured.
    pub fn get(&self, channel_id: &str) -> Option<&ChannelSandbox> {
        self.policies.get(channel_id)
    }

    /// Set or replace the sandbox for a channel.
    pub fn set(&mut self, channel_id: impl Into<String>, sandbox: ChannelSandbox) {
        self.policies.insert(channel_id.into(), sandbox);
    }

    /// Remove the sandbox for a channel.  Returns `true` if one existed.
    pub fn remove(&mut self, channel_id: &str) -> bool {
        self.policies.remove(channel_id).is_some()
    }

    /// Human-readable summary.
    pub fn summary(&self) -> String {
        if self.policies.is_empty() {
            return "sandbox: no channel policies configured".to_string();
        }
        let mut lines = vec![format!(
            "sandbox: {} channel policy/policies",
            self.policies.len()
        )];
        let mut ids: Vec<&str> = self.policies.keys().map(String::as_str).collect();
        ids.sort();
        for id in ids {
            let p = &self.policies[id];
            let label = if p.label.is_empty() {
                id
            } else {
                p.label.as_str()
            };
            let kinds = if p.allowed_tool_kinds.is_empty() {
                "all kinds".to_string()
            } else {
                p.allowed_tool_kinds.join(", ")
            };
            let level = p
                .max_security_level
                .map(|l| format!("max-level={l}"))
                .unwrap_or_else(|| "no level cap".to_string());
            let approval = if p.force_approval {
                " force-approval"
            } else {
                ""
            };
            lines.push(format!("  {id} ({label}): {kinds} {level}{approval}"));
        }
        lines.join("\n")
    }
}

/// Load the sandbox store, evaluate a tool for a given channel, and return the
/// verdict.  Returns `Allow` when no policy is configured for the channel.
pub fn check_channel_tool(
    paths: &PraxisPaths,
    channel_id: &str,
    tool_name: &str,
    tool_kind: ToolKind,
    required_level: u8,
) -> SandboxVerdict {
    let store = match ChannelSandboxStore::load(&paths.sandbox_file) {
        Ok(s) => s,
        Err(e) => {
            log::warn!("sandbox: failed to load store (fail-closed): {e}");
            return SandboxVerdict::Block(format!("sandbox config unreadable: {e}"));
        }
    };
    match store.get(channel_id) {
        None => SandboxVerdict::Allow,
        Some(sandbox) => evaluate_tool(sandbox, tool_name, tool_kind, required_level),
    }
}

fn glob_match(pattern: &str, value: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    if !pattern.contains('*') {
        return pattern == value;
    }
    if let Some(prefix) = pattern.strip_suffix('*') {
        return value.starts_with(prefix);
    }
    if let Some(suffix) = pattern.strip_prefix('*') {
        return value.ends_with(suffix);
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
    use super::*;
    use crate::tools::ToolKind;

    fn tool(kind: ToolKind, level: u8) -> (ToolKind, u8) {
        (kind, level)
    }

    #[test]
    fn strict_sandbox_blocks_shell_tools() {
        let sandbox = ChannelSandbox::strict("test");
        let verdict = evaluate_tool(&sandbox, "shell-cmd", ToolKind::Shell, 1);
        assert!(matches!(verdict, SandboxVerdict::Block(_)));
    }

    #[test]
    fn strict_sandbox_allows_internal_at_level_1() {
        let sandbox = ChannelSandbox::strict("test");
        let verdict = evaluate_tool(&sandbox, "internal-maintenance", ToolKind::Internal, 1);
        assert_eq!(verdict, SandboxVerdict::RequireApproval);
    }

    #[test]
    fn strict_sandbox_blocks_high_level_internal() {
        let sandbox = ChannelSandbox::strict("test");
        let verdict = evaluate_tool(&sandbox, "any-tool", ToolKind::Internal, 2);
        assert!(matches!(verdict, SandboxVerdict::Block(_)));
    }

    #[test]
    fn denied_pattern_blocks_matching_tool() {
        let sandbox = ChannelSandbox {
            denied_tool_name_patterns: vec!["risky-*".to_string()],
            ..Default::default()
        };
        assert!(matches!(
            evaluate_tool(&sandbox, "risky-delete", ToolKind::Shell, 1),
            SandboxVerdict::Block(_)
        ));
        assert_eq!(
            evaluate_tool(&sandbox, "safe-read", ToolKind::Shell, 1),
            SandboxVerdict::Allow
        );
    }

    #[test]
    fn default_sandbox_allows_everything() {
        let sandbox = ChannelSandbox::default();
        assert_eq!(
            evaluate_tool(&sandbox, "any-tool", ToolKind::Http, 3),
            SandboxVerdict::Allow
        );
    }
}
