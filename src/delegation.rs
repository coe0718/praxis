//! Delegation links — explicit directed relationships between Praxis instances
//! or external agents, with concurrency caps and allow/deny lists.
//!
//! A `DelegationLink` declares that this Praxis instance can delegate work to
//! (or receive work from) another agent endpoint.  Links are persisted in
//! `delegation_links.json` and consulted during the Act phase when spawning
//! sub-agents or accepting inbound task requests.
//!
//! ## Link types
//!
//! | Type          | Direction              | Example                            |
//! |---------------|------------------------|------------------------------------|
//! | `Outbound`    | this → remote          | spawn a reviewer sub-agent         |
//! | `Inbound`     | remote → this          | accept tasks from an orchestrator  |
//! | `Bidirectional` | both directions      | peer-to-peer collaboration         |
//!
//! ## Security
//!
//! Each link carries an optional `allow_tasks` list (glob patterns) and
//! `deny_tasks` list.  An empty `allow_tasks` means all tasks are permitted
//! modulo the deny list.  Inbound links that do not pass the allow/deny check
//! are rejected before they reach the approval queue.

use std::{collections::HashMap, fs, path::Path};

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A task received by this agent from a remote agent via the delegation queue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegatedTask {
    pub source: String,
    pub task: String,
    pub link_name: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Write a task to a remote agent endpoint via a delegation link.
///
/// File-based endpoints (absolute paths or `~/…`) are supported: the task is
/// written as a `WakeIntent` into `{endpoint}/delegation_queue.jsonl` so the
/// remote instance picks it up during its next Orient phase.
///
/// HTTP endpoints are not yet supported.
pub fn send_over_link(
    link: &mut DelegationLink,
    task: &str,
    from: &str,
    now: DateTime<Utc>,
) -> Result<()> {
    let endpoint = &link.endpoint;
    if endpoint.starts_with('/') || endpoint.starts_with('~') {
        let remote_dir = Path::new(endpoint.as_str());
        let entry = DelegatedTask {
            source: from.to_string(),
            task: task.to_string(),
            link_name: Some(link.name.clone()),
            created_at: now,
        };
        let queue_path = remote_dir.join("delegation_queue.jsonl");
        if let Some(parent) = queue_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        let line = serde_json::to_string(&entry).context("failed to serialize delegated task")?;
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&queue_path)
            .with_context(|| format!("failed to open {}", queue_path.display()))?;
        use std::io::Write;
        writeln!(file, "{line}")
            .with_context(|| format!("failed to append to {}", queue_path.display()))?;
    } else {
        anyhow::bail!(
            "HTTP delegation endpoints are not yet supported; use an absolute file path: {}",
            endpoint
        );
    }
    link.last_used_at = Some(now);
    Ok(())
}

/// Read and clear all pending inbound delegation tasks from `delegation_queue.jsonl`.
///
/// Each entry is a JSON-encoded `DelegatedTask`. The queue file is truncated
/// after reading so entries are processed exactly once.
pub fn drain_inbound_delegation(queue_path: &Path) -> Result<Vec<DelegatedTask>> {
    if !queue_path.exists() {
        return Ok(Vec::new());
    }
    let raw = fs::read_to_string(queue_path)
        .with_context(|| format!("failed to read {}", queue_path.display()))?;

    let tasks: Vec<DelegatedTask> = raw
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| match serde_json::from_str::<DelegatedTask>(l) {
            Ok(t) => Some(t),
            Err(e) => {
                log::warn!("skipping malformed delegation queue entry: {e}");
                None
            }
        })
        .collect();

    if !tasks.is_empty() {
        // Truncate — entries are consumed exactly once.
        fs::write(queue_path, "")
            .with_context(|| format!("failed to clear {}", queue_path.display()))?;
    }

    Ok(tasks)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LinkDirection {
    Outbound,
    Inbound,
    Bidirectional,
}

impl LinkDirection {
    pub fn can_send(&self) -> bool {
        matches!(self, Self::Outbound | Self::Bidirectional)
    }

    pub fn can_receive(&self) -> bool {
        matches!(self, Self::Inbound | Self::Bidirectional)
    }
}

/// A single configured delegation link.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegationLink {
    /// Human-readable name for this link.
    pub name: String,
    /// Endpoint identifier — URL, agent name, or channel address.
    pub endpoint: String,
    pub direction: LinkDirection,
    /// Maximum concurrent delegated tasks allowed on this link.
    #[serde(default = "default_concurrency")]
    pub max_concurrency: usize,
    /// Glob patterns of task names this link is allowed to carry.
    /// Empty = allow all (subject to deny list).
    #[serde(default)]
    pub allow_tasks: Vec<String>,
    /// Glob patterns of task names this link must never carry.
    #[serde(default)]
    pub deny_tasks: Vec<String>,
    /// Whether this link is currently enabled.
    #[serde(default = "default_true")]
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    #[serde(default)]
    pub last_used_at: Option<DateTime<Utc>>,
}

fn default_concurrency() -> usize {
    1
}
fn default_true() -> bool {
    true
}

impl DelegationLink {
    pub fn new(
        name: impl Into<String>,
        endpoint: impl Into<String>,
        direction: LinkDirection,
    ) -> Self {
        Self {
            name: name.into(),
            endpoint: endpoint.into(),
            direction,
            max_concurrency: 1,
            allow_tasks: Vec::new(),
            deny_tasks: Vec::new(),
            enabled: true,
            created_at: Utc::now(),
            last_used_at: None,
        }
    }

    /// True if `task_name` is permitted on this link.
    pub fn permits(&self, task_name: &str) -> bool {
        if !self.enabled {
            return false;
        }
        // Deny list takes precedence.
        for pattern in &self.deny_tasks {
            if glob_match(pattern, task_name) {
                return false;
            }
        }
        // Allow list: empty = allow all.
        if self.allow_tasks.is_empty() {
            return true;
        }
        self.allow_tasks.iter().any(|p| glob_match(p, task_name))
    }
}

/// The full store — keyed by link name.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DelegationStore {
    #[serde(default)]
    pub links: HashMap<String, DelegationLink>,
    /// Concurrent usage counter per link name.
    #[serde(default)]
    pub active_counts: HashMap<String, usize>,
}

impl DelegationStore {
    pub fn load(path: &Path) -> Result<Self> {
        match fs::read_to_string(path) {
            Ok(raw) => serde_json::from_str(&raw)
                .with_context(|| format!("invalid delegation store at {}", path.display())),
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
            serde_json::to_string_pretty(self).context("failed to serialize delegation store")?;
        fs::write(path, raw).with_context(|| format!("failed to write {}", path.display()))
    }

    pub fn add_link(&mut self, link: DelegationLink) {
        self.links.insert(link.name.clone(), link);
    }

    pub fn remove_link(&mut self, name: &str) -> Option<DelegationLink> {
        self.links.remove(name)
    }

    /// Find all outbound links that permit `task_name` and have capacity.
    pub fn available_outbound(&self, task_name: &str) -> Vec<&DelegationLink> {
        self.links
            .values()
            .filter(|link| {
                link.direction.can_send()
                    && link.permits(task_name)
                    && self.active_counts.get(&link.name).copied().unwrap_or(0)
                        < link.max_concurrency
            })
            .collect()
    }

    /// Increment the active counter for a link (call when delegation starts).
    pub fn acquire(&mut self, link_name: &str) {
        *self.active_counts.entry(link_name.to_string()).or_default() += 1;
    }

    /// Decrement the active counter for a link (call when delegation completes).
    pub fn release(&mut self, link_name: &str) {
        if let Some(count) = self.active_counts.get_mut(link_name) {
            *count = count.saturating_sub(1);
        }
    }

    /// Format a human-readable summary of all links.
    pub fn summary(&self) -> String {
        if self.links.is_empty() {
            return "delegation: no links configured".to_string();
        }
        let mut lines = vec![format!("delegation: {} link(s)", self.links.len())];
        for link in self.links.values() {
            let active = self.active_counts.get(&link.name).copied().unwrap_or(0);
            let status = if link.enabled { "enabled" } else { "disabled" };
            lines.push(format!(
                "  {} ({:?}) → {} [{status}] {}/{} active",
                link.name, link.direction, link.endpoint, active, link.max_concurrency
            ));
        }
        lines.join("\n")
    }
}

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
