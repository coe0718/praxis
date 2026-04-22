//! Wake-on-intent — approved interrupt-style session waking.
//!
//! External systems (webhooks, cron, operator tools) can request an immediate
//! Praxis session by writing a `wake_intent.json` file to the data directory.
//! The scheduler checks for this file before deciding whether to defer the
//! current window to quiet hours.
//!
//! A wake intent carries:
//! - `reason` — human-readable description of what triggered the wake
//! - `source` — which system produced it ("webhook", "telegram", "goal", etc.)
//! - `task` — optional task string to inject into the upcoming session
//! - `priority` — "urgent" (bypass quiet-hours check) or "normal"
//! - `created_at` — timestamp so stale intents can be discarded
//!
//! The intent is consumed (deleted) once the session runs so it only triggers
//! one wake.

pub mod schedule;

use std::{fs, path::Path};

use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

const FILENAME: &str = "wake_intent.json";

/// How old a wake intent may be before it is considered stale and discarded.
const MAX_AGE_HOURS: i64 = 24;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WakeIntent {
    pub reason: String,
    pub source: String,
    #[serde(default)]
    pub task: Option<String>,
    #[serde(default = "default_priority")]
    pub priority: WakePriority,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum WakePriority {
    /// Bypass quiet-hours check.
    Urgent,
    /// Respect quiet-hours like any other session.
    Normal,
}

fn default_priority() -> WakePriority {
    WakePriority::Normal
}

impl WakeIntent {
    pub fn new(reason: impl Into<String>, source: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
            source: source.into(),
            task: None,
            priority: WakePriority::Normal,
            created_at: Utc::now(),
        }
    }

    pub fn with_task(mut self, task: impl Into<String>) -> Self {
        self.task = Some(task.into());
        self
    }

    pub fn urgent(mut self) -> Self {
        self.priority = WakePriority::Urgent;
        self
    }

    pub fn is_urgent(&self) -> bool {
        self.priority == WakePriority::Urgent
    }
}

/// Write a wake intent to the data directory.
pub fn request_wake(data_dir: &Path, intent: &WakeIntent) -> Result<()> {
    let path = data_dir.join(FILENAME);
    let raw = serde_json::to_string_pretty(intent).context("failed to serialize wake intent")?;
    fs::write(&path, raw)
        .with_context(|| format!("failed to write wake intent to {}", path.display()))
}

/// Read and consume the pending wake intent, if one exists.
/// Returns `None` if no intent is pending or if the intent is stale.
/// The file is deleted after reading.
pub fn consume_intent(data_dir: &Path) -> Result<Option<WakeIntent>> {
    let path = data_dir.join(FILENAME);

    let raw = match fs::read_to_string(&path) {
        Ok(content) => content,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => {
            return Err(anyhow::Error::new(e).context(format!(
                "failed to read wake intent from {}",
                path.display()
            )));
        }
    };

    // Delete immediately — the intent is consumed regardless of whether we
    // can parse it, so a corrupt file does not loop.
    let _ = fs::remove_file(&path);

    let intent: WakeIntent = match serde_json::from_str(&raw) {
        Ok(i) => i,
        Err(e) => {
            log::warn!("discarding corrupt wake intent file: {e}");
            return Ok(None);
        }
    };

    // Discard stale intents.
    let age = Utc::now().signed_duration_since(intent.created_at);
    if age > Duration::hours(MAX_AGE_HOURS) {
        log::warn!(
            "discarding stale wake intent ({:.0}h old, max {MAX_AGE_HOURS}h)",
            age.num_minutes() as f64 / 60.0
        );
        return Ok(None);
    }

    Ok(Some(intent))
}

/// Check whether a wake intent is pending (without consuming it).
pub fn is_pending(data_dir: &Path) -> bool {
    data_dir.join(FILENAME).exists()
}

/// Format a wake intent summary for logging.
pub fn format_summary(intent: &WakeIntent) -> String {
    let priority = if intent.is_urgent() {
        "urgent"
    } else {
        "normal"
    };
    let task = intent
        .task
        .as_deref()
        .map(|t| format!(" task=\"{t}\""))
        .unwrap_or_default();
    format!(
        "[wake-on-intent] source={} priority={priority}{task} reason=\"{}\"",
        intent.source, intent.reason
    )
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;

    #[test]
    fn round_trips_a_wake_intent() {
        let tmp = tempdir().unwrap();
        let intent = WakeIntent::new("PR merged", "webhook")
            .with_task("run tests")
            .urgent();

        request_wake(tmp.path(), &intent).unwrap();
        assert!(is_pending(tmp.path()));

        let loaded = consume_intent(tmp.path()).unwrap().unwrap();
        assert_eq!(loaded.reason, "PR merged");
        assert_eq!(loaded.source, "webhook");
        assert_eq!(loaded.task.as_deref(), Some("run tests"));
        assert!(loaded.is_urgent());

        // Consumed — file should be gone.
        assert!(!is_pending(tmp.path()));
    }

    #[test]
    fn consume_returns_none_when_absent() {
        let tmp = tempdir().unwrap();
        assert!(consume_intent(tmp.path()).unwrap().is_none());
    }

    #[test]
    fn discards_stale_intents() {
        let tmp = tempdir().unwrap();
        // Write an intent timestamped 25 hours ago.
        let intent = WakeIntent {
            reason: "old event".to_string(),
            source: "test".to_string(),
            task: None,
            priority: WakePriority::Normal,
            created_at: Utc::now() - Duration::hours(25),
        };
        request_wake(tmp.path(), &intent).unwrap();

        let result = consume_intent(tmp.path()).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn format_summary_includes_key_fields() {
        let intent = WakeIntent::new("deploy complete", "ci").urgent();
        let s = format_summary(&intent);
        assert!(s.contains("source=ci"));
        assert!(s.contains("urgent"));
        assert!(s.contains("deploy complete"));
    }
}
