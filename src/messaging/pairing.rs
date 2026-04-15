//! Sender pairing — lets unknown Telegram chats request access.
//!
//! When a message arrives from a chat ID that is not in the static
//! allow-list, the bot generates a 6-digit one-time code, stores the
//! code and the queued message, and sends the code to the operator's
//! primary chat.  All further input from the same unknown chat is
//! silently dropped until the operator sends `/approve-sender <code>`.
//!
//! On approval the queued message is returned for immediate processing
//! and the chat ID is written into the approved list so future messages
//! pass through without re-verification.

use std::{collections::HashMap, fs, path::Path};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// A single pending pairing request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingEntry {
    /// Six-digit one-time code sent to the operator.
    pub code: String,
    /// The first message the unknown chat sent (replayed on approval).
    pub queued_text: String,
}

/// Persistent pairing state written to `sender_pairing.json`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PairingStore {
    /// Chat IDs approved via the pairing flow (supplements the static allow-list).
    #[serde(default)]
    pub approved_chat_ids: Vec<i64>,
    /// In-flight pairing requests keyed by chat ID (as string).
    #[serde(default)]
    pub pending: HashMap<String, PendingEntry>,
}

impl PairingStore {
    pub fn load(path: &Path) -> Result<Self> {
        match fs::read_to_string(path) {
            Ok(raw) => serde_json::from_str(&raw)
                .with_context(|| format!("invalid pairing store at {}", path.display())),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(e).with_context(|| format!("failed to read {}", path.display())),
        }
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        let raw = serde_json::to_string_pretty(self).context("failed to serialize pairing store")?;
        fs::write(path, raw).with_context(|| format!("failed to write {}", path.display()))
    }

    /// True if the chat has already been approved (dynamically).
    pub fn is_approved(&self, chat_id: i64) -> bool {
        self.approved_chat_ids.contains(&chat_id)
    }

    /// True if a pairing code has already been sent to this chat.
    pub fn is_pending(&self, chat_id: i64) -> bool {
        self.pending.contains_key(&chat_id.to_string())
    }

    /// Start a pairing request for a new chat.  Returns the generated code.
    pub fn initiate(&mut self, chat_id: i64, queued_text: &str) -> String {
        let code = generate_code();
        self.pending.insert(
            chat_id.to_string(),
            PendingEntry {
                code: code.clone(),
                queued_text: queued_text.to_string(),
            },
        );
        code
    }

    /// Attempt to approve a chat by matching `code`.
    ///
    /// Returns the queued message text if the code matched, or `None` if not.
    pub fn approve_by_code(&mut self, code: &str) -> Option<(i64, String)> {
        let key = self
            .pending
            .iter()
            .find(|(_, entry)| entry.code == code)
            .map(|(k, _)| k.clone())?;

        let entry = self.pending.remove(&key)?;
        let chat_id: i64 = key.parse().ok()?;
        self.approved_chat_ids.push(chat_id);
        Some((chat_id, entry.queued_text))
    }
}

fn generate_code() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    // Simple non-cryptographic 6-digit code derived from nanosecond timestamp.
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    format!("{:06}", nanos % 1_000_000)
}
