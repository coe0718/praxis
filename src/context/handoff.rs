//! Context handoff notes — emitted when in-session context pressure exceeds
//! the 50% threshold defined in the design spec.
//!
//! A handoff note records what the agent was working on, what it has completed,
//! and what remains, so a fresh context window can resume without losing state.
//!
//! The note is written to `data_dir/handoff_note.json`.  It is overwritten on
//! each check so the latest state is always what matters.

use std::{fs, path::Path};

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Fraction of the context budget at which a handoff note is written.
pub const HANDOFF_THRESHOLD: f32 = 0.50;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandoffNote {
    pub goal: Option<String>,
    pub completed: Vec<String>,
    pub remaining: Vec<String>,
    pub key_facts: Vec<String>,
    pub do_not_forget: Vec<String>,
    pub context_pressure_pct: f32,
    pub recorded_at: DateTime<Utc>,
}

impl HandoffNote {
    pub fn new(
        goal: Option<String>,
        action_summary: Option<&str>,
        pressure_pct: f32,
        now: DateTime<Utc>,
    ) -> Self {
        let mut key_facts = Vec::new();
        if let Some(summary) = action_summary {
            if !summary.is_empty() {
                key_facts.push(summary.to_string());
            }
        }
        Self {
            goal,
            completed: Vec::new(),
            remaining: Vec::new(),
            key_facts,
            do_not_forget: Vec::new(),
            context_pressure_pct: pressure_pct,
            recorded_at: now,
        }
    }
}

/// Write a handoff note to `data_dir/handoff_note.json` when context pressure
/// exceeds the threshold.  Returns `true` if a note was written.
pub fn write_if_needed(
    data_dir: &Path,
    pressure_pct: f32,
    goal: Option<&str>,
    action_summary: Option<&str>,
    now: DateTime<Utc>,
) -> Result<bool> {
    if pressure_pct < HANDOFF_THRESHOLD {
        return Ok(false);
    }

    let note = HandoffNote::new(goal.map(ToString::to_string), action_summary, pressure_pct, now);
    let path = data_dir.join("handoff_note.json");
    let raw = serde_json::to_string_pretty(&note).context("failed to serialize handoff note")?;
    fs::write(&path, raw)
        .with_context(|| format!("failed to write handoff note to {}", path.display()))?;

    Ok(true)
}

/// Load the most recent handoff note, if one exists.
pub fn load(data_dir: &Path) -> Option<HandoffNote> {
    let path = data_dir.join("handoff_note.json");
    let raw = fs::read_to_string(&path).ok()?;
    serde_json::from_str(&raw).ok()
}

/// Remove the handoff note after it has been consumed by a fresh Orient.
pub fn clear(data_dir: &Path) -> Result<()> {
    let path = data_dir.join("handoff_note.json");
    if path.exists() {
        fs::remove_file(&path)
            .with_context(|| format!("failed to clear handoff note at {}", path.display()))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;
    use tempfile::tempdir;

    use super::*;

    #[test]
    fn writes_handoff_note_when_pressure_exceeds_threshold() {
        let tmp = tempdir().unwrap();
        let now = chrono::Utc.with_ymd_and_hms(2026, 4, 14, 8, 0, 0).unwrap();

        let wrote = write_if_needed(tmp.path(), 0.55, Some("G-042: ship memory"), Some("completed module"), now).unwrap();
        assert!(wrote);

        let note = load(tmp.path()).unwrap();
        assert_eq!(note.goal.as_deref(), Some("G-042: ship memory"));
        assert!(!note.key_facts.is_empty());
        assert!((note.context_pressure_pct - 0.55).abs() < f32::EPSILON);
    }

    #[test]
    fn skips_note_when_pressure_is_below_threshold() {
        let tmp = tempdir().unwrap();
        let now = chrono::Utc.with_ymd_and_hms(2026, 4, 14, 8, 0, 0).unwrap();

        let wrote = write_if_needed(tmp.path(), 0.40, None, None, now).unwrap();
        assert!(!wrote);
        assert!(load(tmp.path()).is_none());
    }

    #[test]
    fn clear_removes_existing_note() {
        let tmp = tempdir().unwrap();
        let now = chrono::Utc.with_ymd_and_hms(2026, 4, 14, 8, 0, 0).unwrap();
        write_if_needed(tmp.path(), 0.60, None, None, now).unwrap();
        assert!(load(tmp.path()).is_some());

        clear(tmp.path()).unwrap();
        assert!(load(tmp.path()).is_none());
    }
}
