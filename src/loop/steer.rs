//! #16 `/steer` Mid-Run Nudges
//!
//! A mechanism to inject notes that the agent sees after the next tool call.
//! Steer notes are stored in a thread-safe queue. When processing a tool
//! response, the agent loop checks for pending steer notes and injects them
//! into the agent context as an additional system-level nudge.
//!
//! Usage:
//!   - Telegram: `/steer <text>`
//!   - CLI:      `praxis wake --task "steer:<text>"`
//!
//! The `SteerQueue` is a process-wide singleton backed by a file (`steer_queue.json`)
//! so that CLI invocations can push notes that the running daemon picks up.

use std::collections::VecDeque;
use std::fs;
use std::path::Path;
use std::sync::Mutex;

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A single steering note injected by the operator mid-run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SteerNote {
    /// The nudge text — injected verbatim into the agent context.
    pub text: String,
    /// When this note was created.
    pub created_at: DateTime<Utc>,
    /// Source that created the note (e.g. "telegram", "cli").
    pub source: String,
    /// Whether this note has been consumed by the agent loop.
    #[serde(default)]
    pub consumed: bool,
}

impl SteerNote {
    /// Create a new steer note from the given text and source.
    pub fn new(text: impl Into<String>, source: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            created_at: Utc::now(),
            source: source.into(),
            consumed: false,
        }
    }
}

/// In-memory thread-safe queue for steer notes within a single process.
pub struct SteerQueue {
    queue: Mutex<VecDeque<SteerNote>>,
}

impl SteerQueue {
    /// Create a new empty steer queue.
    pub fn new() -> Self {
        Self {
            queue: Mutex::new(VecDeque::new()),
        }
    }

    /// Push a steer note onto the queue.
    pub fn push(&self, note: SteerNote) {
        let mut q = self.queue.lock().unwrap_or_else(|e| e.into_inner());
        q.push_back(note);
        log::info!("steer note queued ({} pending)", q.len());
    }

    /// Drain all pending (unconsumed) steer notes, marking them as consumed.
    /// Returns notes in FIFO order. Called by the agent loop after a tool
    /// response to inject mid-run nudges.
    pub fn drain_pending(&self) -> Vec<SteerNote> {
        let mut q = self.queue.lock().unwrap_or_else(|e| e.into_inner());
        let pending: Vec<SteerNote> = q.iter().filter(|n| !n.consumed).cloned().collect();
        for note in q.iter_mut() {
            note.consumed = true;
        }
        if !pending.is_empty() {
            log::info!("drained {} steer note(s) for injection", pending.len());
        }
        pending
    }

    /// Return the number of pending (unconsumed) notes.
    pub fn pending_count(&self) -> usize {
        let q = self.queue.lock().unwrap_or_else(|e| e.into_inner());
        q.iter().filter(|n| !n.consumed).count()
    }

    /// Purge consumed notes from the queue to keep memory bounded.
    pub fn purge_consumed(&self) {
        let mut q = self.queue.lock().unwrap_or_else(|e| e.into_inner());
        q.retain(|n| !n.consumed);
    }
}

impl Default for SteerQueue {
    fn default() -> Self {
        Self::new()
    }
}

// ── File-backed persistence ──────────────────────────────────────────────────
// Allows CLI `/steer` commands to push notes that a running daemon picks up.

/// Persistent file-backed steer note store (for cross-process communication).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SteerFileStore {
    #[serde(default)]
    pub notes: Vec<SteerNote>,
}

impl SteerFileStore {
    /// Load the steer store from disk, returning default if absent.
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let raw = fs::read_to_string(path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        serde_json::from_str(&raw).with_context(|| format!("invalid JSON in {}", path.display()))
    }

    /// Persist to disk.
    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        let raw = serde_json::to_string_pretty(self).context("failed to serialize steer store")?;
        fs::write(path, raw).with_context(|| format!("failed to write {}", path.display()))
    }

    /// Push a new steer note and persist.
    pub fn push_note(path: &Path, text: &str, source: &str) -> Result<SteerNote> {
        let mut store = Self::load(path)?;
        let note = SteerNote::new(text, source);
        store.notes.push(note.clone());
        store.save(path)?;
        Ok(note)
    }

    /// Drain all unconsumed notes, marking them consumed and persisting.
    pub fn drain_pending(path: &Path) -> Result<Vec<SteerNote>> {
        let mut store = Self::load(path)?;
        let pending: Vec<SteerNote> = store.notes.iter().filter(|n| !n.consumed).cloned().collect();
        for note in store.notes.iter_mut() {
            note.consumed = true;
        }
        // Purge old consumed notes (keep last 100).
        if store.notes.len() > 100 {
            let consumed_count = store.notes.iter().filter(|n| n.consumed).count();
            if consumed_count > 50 {
                store.notes.retain(|n| !n.consumed);
            }
        }
        store.save(path)?;
        Ok(pending)
    }
}

/// Format steer notes into a context injection string for the agent.
///
/// Returns `None` if there are no notes to inject.
pub fn format_steer_injection(notes: &[SteerNote]) -> Option<String> {
    if notes.is_empty() {
        return None;
    }
    let mut parts = vec![
        "═══ OPERATOR STEERING NOTE ═══".to_string(),
        "The operator has sent mid-run nudges that you should incorporate now:".to_string(),
        String::new(),
    ];
    for (i, note) in notes.iter().enumerate() {
        parts.push(format!("{}. [{}] {}", i + 1, note.source, note.text));
    }
    parts.push(String::new());
    parts.push("Incorporate these directions into your current work.".to_string());
    Some(parts.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steer_queue_push_and_drain() {
        let q = SteerQueue::new();
        assert_eq!(q.pending_count(), 0);

        q.push(SteerNote::new("focus on tests", "cli"));
        q.push(SteerNote::new("also check errors", "telegram"));
        assert_eq!(q.pending_count(), 2);

        let pending = q.drain_pending();
        assert_eq!(pending.len(), 2);
        assert_eq!(pending[0].text, "focus on tests");
        assert_eq!(pending[1].text, "also check errors");
        assert_eq!(q.pending_count(), 0);
    }

    #[test]
    fn steer_queue_drain_idempotent() {
        let q = SteerQueue::new();
        q.push(SteerNote::new("hello", "test"));
        let _first = q.drain_pending();
        let second = q.drain_pending();
        assert!(second.is_empty());
    }

    #[test]
    fn format_steer_injection_empty() {
        assert!(format_steer_injection(&[]).is_none());
    }

    #[test]
    fn format_steer_injection_single() {
        let notes = vec![SteerNote::new("do the thing", "cli")];
        let injection = format_steer_injection(&notes).unwrap();
        assert!(injection.contains("OPERATOR STEERING NOTE"));
        assert!(injection.contains("do the thing"));
    }

    #[test]
    fn file_store_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("steer_queue.json");

        let note = SteerFileStore::push_note(&path, "test nudge", "unit-test").unwrap();
        assert_eq!(note.text, "test nudge");
        assert!(!note.consumed);

        let pending = SteerFileStore::drain_pending(&path).unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].text, "test nudge");

        // After drain, no more pending.
        let pending2 = SteerFileStore::drain_pending(&path).unwrap();
        assert!(pending2.is_empty());
    }
}
