//! Central message bus — the transport spine for all adapter-sourced events.
//!
//! Messaging adapters (Telegram, webhooks, voice) publish normalized
//! [`BusEvent`]s onto the bus.  The agent loop, approval queue, dashboard, and
//! hook system subscribe to the bus rather than calling each integration
//! directly.
//!
//! This keeps the **heartbeat** (liveness probe, `heartbeat.json`) strictly
//! separate from the **message bus** (transport spine, `bus.jsonl`).
//! Heartbeat records process health; the bus records inbound user intent.
//!
//! # Implementations
//! - [`FileBus`] — appends events to `bus.jsonl`; `drain()` reads and clears.
//! - [`NullBus`] — no-op for tests and contexts that don't need the bus.

use std::{
    fs::{self, OpenOptions},
    io::Write as _,
    path::Path,
};

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ── Event type ────────────────────────────────────────────────────────────────

/// A normalized inbound event published by any adapter onto the message bus.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BusEvent {
    /// Event kind (e.g. `"message"`, `"voice"`, `"file"`, `"command"`).
    pub kind: String,
    /// Adapter that produced the event (e.g. `"telegram"`, `"webhook"`).
    pub source: String,
    /// Conversation or channel identifier — opaque to the bus.
    pub conversation_id: String,
    /// Sender identifier — opaque to the bus.
    pub sender_id: String,
    /// Payload: the normalized text or content of the event.
    pub payload: String,
    pub published_at: DateTime<Utc>,
}

impl BusEvent {
    pub fn new(
        kind: impl Into<String>,
        source: impl Into<String>,
        conversation_id: impl Into<String>,
        sender_id: impl Into<String>,
        payload: impl Into<String>,
    ) -> Self {
        Self {
            kind: kind.into(),
            source: source.into(),
            conversation_id: conversation_id.into(),
            sender_id: sender_id.into(),
            payload: payload.into(),
            published_at: Utc::now(),
        }
    }
}

// ── Trait ─────────────────────────────────────────────────────────────────────

/// A message bus that adapters publish to and the agent loop drains.
pub trait MessageBus: Send + Sync {
    /// Publish one event onto the bus.
    fn publish(&self, event: &BusEvent) -> Result<()>;

    /// Drain all pending events from the bus, returning them in order.
    /// After draining, the bus is empty.
    fn drain(&self) -> Result<Vec<BusEvent>>;

    /// Return all pending events without consuming them.
    fn peek(&self) -> Result<Vec<BusEvent>>;
}

// ── FileBus ───────────────────────────────────────────────────────────────────

/// A [`MessageBus`] backed by an append-only JSONL file.
///
/// `publish()` appends a single JSON line.
/// `drain()` reads the file, clears it, and returns all events.
pub struct FileBus {
    path: std::path::PathBuf,
}

impl FileBus {
    pub fn new(path: impl Into<std::path::PathBuf>) -> Self {
        Self { path: path.into() }
    }
}

impl MessageBus for FileBus {
    fn publish(&self, event: &BusEvent) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .with_context(|| format!("failed to open bus file {}", self.path.display()))?;
        let line = serde_json::to_string(event).context("failed to serialize bus event")?;
        writeln!(file, "{line}")
            .with_context(|| format!("failed to write to {}", self.path.display()))
    }

    fn drain(&self) -> Result<Vec<BusEvent>> {
        let events = self.peek()?;
        if !events.is_empty() && self.path.exists() {
            fs::write(&self.path, "")
                .with_context(|| format!("failed to clear bus file {}", self.path.display()))?;
        }
        Ok(events)
    }

    fn peek(&self) -> Result<Vec<BusEvent>> {
        read_events(&self.path)
    }
}

fn read_events(path: &Path) -> Result<Vec<BusEvent>> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let raw =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    let mut events = Vec::new();
    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Ok(event) = serde_json::from_str::<BusEvent>(line) {
            events.push(event);
        }
    }
    Ok(events)
}

// ── NullBus ───────────────────────────────────────────────────────────────────

/// A no-op [`MessageBus`] for tests and contexts that don't use the bus.
#[derive(Debug, Default, Clone, Copy)]
pub struct NullBus;

impl MessageBus for NullBus {
    fn publish(&self, _event: &BusEvent) -> Result<()> {
        Ok(())
    }

    fn drain(&self) -> Result<Vec<BusEvent>> {
        Ok(Vec::new())
    }

    fn peek(&self) -> Result<Vec<BusEvent>> {
        Ok(Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::{BusEvent, FileBus, MessageBus, NullBus};

    fn event(kind: &str) -> BusEvent {
        BusEvent::new(kind, "telegram", "chat:42", "user:1", "hello")
    }

    #[test]
    fn file_bus_publish_and_drain() {
        let tmp = tempdir().unwrap();
        let bus = FileBus::new(tmp.path().join("bus.jsonl"));

        bus.publish(&event("message")).unwrap();
        bus.publish(&event("command")).unwrap();

        let events = bus.drain().unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].kind, "message");
        assert_eq!(events[1].kind, "command");

        // After drain, bus is empty.
        let events2 = bus.drain().unwrap();
        assert!(events2.is_empty());
    }

    #[test]
    fn file_bus_peek_does_not_consume() {
        let tmp = tempdir().unwrap();
        let bus = FileBus::new(tmp.path().join("bus.jsonl"));

        bus.publish(&event("message")).unwrap();

        let peeked = bus.peek().unwrap();
        assert_eq!(peeked.len(), 1);

        let peeked_again = bus.peek().unwrap();
        assert_eq!(peeked_again.len(), 1);
    }

    #[test]
    fn null_bus_is_always_empty() {
        let bus = NullBus;
        bus.publish(&event("message")).unwrap();
        assert!(bus.drain().unwrap().is_empty());
    }
}
