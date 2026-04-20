//! Per-conversation activation modes.
//!
//! Shared channels and group contexts need per-session activation modes such
//! as mention-only, thread-only, or always-listening.  The mode belongs to
//! the conversation context, not a single global toggle.
//!
//! Modes are persisted to `activation.json` so they survive across sessions.

use std::{collections::HashMap, fs, path::Path};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// How Praxis responds to inbound messages in a given conversation.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActivationMode {
    /// Respond only when directly mentioned (`@praxis` or equivalent).
    #[default]
    MentionOnly,
    /// Respond only within the originating thread, not to new top-level posts.
    ThreadOnly,
    /// Respond to every message in the conversation.
    AlwaysListening,
}

impl std::fmt::Display for ActivationMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MentionOnly => f.write_str("mention_only"),
            Self::ThreadOnly => f.write_str("thread_only"),
            Self::AlwaysListening => f.write_str("always_listening"),
        }
    }
}

/// Persistent store of per-conversation activation modes.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ActivationStore {
    /// Maps conversation ID → activation mode.
    pub modes: HashMap<String, ActivationMode>,
}

impl ActivationStore {
    /// Load from `activation.json`, returning an empty store if absent.
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let raw = fs::read_to_string(path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        serde_json::from_str(&raw)
            .with_context(|| format!("invalid activation JSON in {}", path.display()))
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        let raw =
            serde_json::to_string_pretty(self).context("failed to serialize activation store")?;
        fs::write(path, raw).with_context(|| format!("failed to write {}", path.display()))
    }

    /// Return the activation mode for a conversation, defaulting to
    /// [`ActivationMode::MentionOnly`] if none is set.
    pub fn get(&self, conversation_id: &str) -> ActivationMode {
        self.modes.get(conversation_id).copied().unwrap_or_default()
    }

    /// Set the activation mode for a conversation.
    pub fn set(&mut self, conversation_id: impl Into<String>, mode: ActivationMode) {
        self.modes.insert(conversation_id.into(), mode);
    }

    /// Remove the activation mode for a conversation (resets to default).
    pub fn remove(&mut self, conversation_id: &str) {
        self.modes.remove(conversation_id);
    }

    /// Check whether Praxis should respond to a message in this conversation.
    ///
    /// `is_mention` — true if the message directly mentions the agent.
    /// `is_thread_reply` — true if the message is a reply within a thread.
    pub fn should_respond(
        &self,
        conversation_id: &str,
        is_mention: bool,
        is_thread_reply: bool,
    ) -> bool {
        match self.get(conversation_id) {
            ActivationMode::MentionOnly => is_mention,
            ActivationMode::ThreadOnly => is_thread_reply || is_mention,
            ActivationMode::AlwaysListening => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::{ActivationMode, ActivationStore};

    #[test]
    fn defaults_to_mention_only() {
        let store = ActivationStore::default();
        assert_eq!(store.get("chat:42"), ActivationMode::MentionOnly);
    }

    #[test]
    fn set_and_get_round_trips_through_file() {
        let tmp = tempdir().unwrap();
        let path = tmp.path().join("activation.json");

        let mut store = ActivationStore::default();
        store.set("chat:42", ActivationMode::AlwaysListening);
        store.save(&path).unwrap();

        let loaded = ActivationStore::load(&path).unwrap();
        assert_eq!(loaded.get("chat:42"), ActivationMode::AlwaysListening);
        assert_eq!(loaded.get("chat:99"), ActivationMode::MentionOnly);
    }

    #[test]
    fn mention_only_requires_mention() {
        let mut store = ActivationStore::default();
        store.set("c", ActivationMode::MentionOnly);
        assert!(store.should_respond("c", true, false));
        assert!(!store.should_respond("c", false, true));
    }

    #[test]
    fn thread_only_responds_to_replies_and_mentions() {
        let mut store = ActivationStore::default();
        store.set("c", ActivationMode::ThreadOnly);
        assert!(store.should_respond("c", false, true));
        assert!(store.should_respond("c", true, false));
        assert!(!store.should_respond("c", false, false));
    }

    #[test]
    fn always_listening_responds_to_everything() {
        let mut store = ActivationStore::default();
        store.set("c", ActivationMode::AlwaysListening);
        assert!(store.should_respond("c", false, false));
    }

    #[test]
    fn remove_resets_to_default() {
        let mut store = ActivationStore::default();
        store.set("c", ActivationMode::AlwaysListening);
        store.remove("c");
        assert_eq!(store.get("c"), ActivationMode::MentionOnly);
    }
}
