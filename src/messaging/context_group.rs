//! Per-context-group isolation — scopes short-horizon memory and context to
//! individual conversations or channels.
//!
//! Each conversation (identified by its `conversation_id`, e.g. a Telegram
//! `chat_id` string) maintains its own `ContextGroupState`:
//!
//! - `pinned_memory_ids`: memory IDs that have been tagged as relevant to this
//!   conversation; loaded first during Orient when the conversation is active.
//! - `pinned_tags`: free-form tag strings written when memories are captured
//!   during a messaging session.
//!
//! The group store is a flat JSON file (`context_groups.json`) keyed by
//! conversation ID.

use std::{collections::HashMap, fs, path::Path};

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// State maintained for one conversation group.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ContextGroupState {
    /// Memory IDs pinned to this conversation.
    #[serde(default)]
    pub pinned_memory_ids: Vec<i64>,
    /// Tag strings added during sessions in this conversation.
    #[serde(default)]
    pub pinned_tags: Vec<String>,
    /// When this group was first seen.
    pub first_seen_at: Option<DateTime<Utc>>,
    /// When this group was last active.
    pub last_active_at: Option<DateTime<Utc>>,
}

impl ContextGroupState {
    /// Record that the group is active now.
    pub fn touch(&mut self, now: DateTime<Utc>) {
        if self.first_seen_at.is_none() {
            self.first_seen_at = Some(now);
        }
        self.last_active_at = Some(now);
    }

    /// Add a memory ID if not already present.
    pub fn pin_memory(&mut self, id: i64) {
        if !self.pinned_memory_ids.contains(&id) {
            self.pinned_memory_ids.push(id);
        }
    }

    /// Add a tag if not already present.
    pub fn pin_tag(&mut self, tag: impl Into<String>) {
        let tag = tag.into();
        if !self.pinned_tags.contains(&tag) {
            self.pinned_tags.push(tag);
        }
    }
}

/// The full store persisted to `context_groups.json`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ContextGroupStore {
    #[serde(default)]
    pub groups: HashMap<String, ContextGroupState>,
}

impl ContextGroupStore {
    pub fn load(path: &Path) -> Result<Self> {
        match fs::read_to_string(path) {
            Ok(raw) => serde_json::from_str(&raw)
                .with_context(|| format!("invalid context group store at {}", path.display())),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(e).with_context(|| format!("failed to read {}", path.display())),
        }
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        let raw = serde_json::to_string_pretty(self)
            .context("failed to serialize context group store")?;
        fs::write(path, raw).with_context(|| format!("failed to write {}", path.display()))
    }

    /// Get or create the state for a conversation.
    pub fn get_or_create(&mut self, conversation_id: &str) -> &mut ContextGroupState {
        self.groups.entry(conversation_id.to_string()).or_default()
    }

    /// Return an existing group without creating it.
    pub fn get(&self, conversation_id: &str) -> Option<&ContextGroupState> {
        self.groups.get(conversation_id)
    }

    /// Mark a conversation as active at `now`.
    pub fn touch(&mut self, conversation_id: &str, now: DateTime<Utc>) {
        self.get_or_create(conversation_id).touch(now);
    }

    /// Pin a memory to a conversation.
    pub fn pin_memory(&mut self, conversation_id: &str, memory_id: i64) {
        self.get_or_create(conversation_id).pin_memory(memory_id);
    }

    /// Pin a tag to a conversation.
    pub fn pin_tag(&mut self, conversation_id: &str, tag: impl Into<String>) {
        self.get_or_create(conversation_id).pin_tag(tag);
    }

    /// All memory IDs pinned to the conversation, deduplicated.
    pub fn pinned_memory_ids(&self, conversation_id: &str) -> Vec<i64> {
        self.get(conversation_id)
            .map(|g| g.pinned_memory_ids.clone())
            .unwrap_or_default()
    }

    /// Prune groups that have been inactive for more than `max_idle_days`.
    pub fn prune_idle(&mut self, max_idle_days: i64, now: DateTime<Utc>) {
        use chrono::Duration;
        let cutoff = now - Duration::days(max_idle_days);
        self.groups
            .retain(|_, state| state.last_active_at.map(|ts| ts > cutoff).unwrap_or(false));
    }
}
