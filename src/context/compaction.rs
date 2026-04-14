//! Interactive and automatic compaction.
//!
//! Two distinct mechanisms:
//!
//! **Interactive compaction** (`/compact`) — operator-driven reset. The
//! operator runs `praxis compact` to request a fresh context window.  The
//! request is written to `compaction.json`.  On the next `orient()`, the
//! runtime detects it, clears it, and starts the session context from scratch
//! (the existing `handoff_note.json` becomes the only carryover).
//!
//! **Automatic compaction** — threshold-triggered.  When in-session context
//! pressure reaches `AUTO_COMPACT_THRESHOLD` (80 %), the runtime writes a
//! compaction request automatically so the *next* session opens cleanly.  This
//! is distinct from the 50 % handoff-note rule: the handoff note records
//! *what was happening*; the compaction request tells the runtime to *reset the
//! window*.

use std::{fs, path::Path};

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Context pressure fraction at which automatic compaction is triggered.
pub const AUTO_COMPACT_THRESHOLD: f32 = 0.80;

const FILENAME: &str = "compaction.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactionRequest {
    /// How the compaction was triggered.
    pub trigger: CompactionTrigger,
    /// Context pressure at the time the request was made (0.0–1.0).
    /// `None` for operator-initiated compactions.
    pub pressure_pct: Option<f32>,
    /// Goal active when the request was made.
    pub goal: Option<String>,
    pub requested_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CompactionTrigger {
    /// Operator explicitly ran `praxis compact`.
    Operator,
    /// Automatic threshold-based trigger.
    Automatic,
}

impl CompactionRequest {
    pub fn operator(goal: Option<String>, now: DateTime<Utc>) -> Self {
        Self {
            trigger: CompactionTrigger::Operator,
            pressure_pct: None,
            goal,
            requested_at: now,
        }
    }

    pub fn automatic(pressure_pct: f32, goal: Option<String>, now: DateTime<Utc>) -> Self {
        Self {
            trigger: CompactionTrigger::Automatic,
            pressure_pct: Some(pressure_pct),
            goal,
            requested_at: now,
        }
    }
}

/// Write a compaction request to the data directory.
pub fn request_compact(data_dir: &Path, req: &CompactionRequest) -> Result<()> {
    let path = data_dir.join(FILENAME);
    let raw = serde_json::to_string_pretty(req).context("failed to serialize compaction request")?;
    fs::write(&path, raw)
        .with_context(|| format!("failed to write compaction request to {}", path.display()))
}

/// Read and consume the pending compaction request, if one exists.
/// The file is deleted immediately after reading.
pub fn consume_compact(data_dir: &Path) -> Result<Option<CompactionRequest>> {
    let path = data_dir.join(FILENAME);
    if !path.exists() {
        return Ok(None);
    }
    let raw = fs::read_to_string(&path)
        .with_context(|| format!("failed to read compaction request from {}", path.display()))?;
    let _ = fs::remove_file(&path);
    Ok(serde_json::from_str(&raw).ok())
}

/// Check whether a compaction request is pending (without consuming it).
pub fn is_pending(data_dir: &Path) -> bool {
    data_dir.join(FILENAME).exists()
}

/// Write an automatic compaction request when pressure exceeds the threshold.
/// Returns `true` if a request was written.
pub fn compact_if_needed(
    data_dir: &Path,
    pressure_pct: f32,
    goal: Option<&str>,
    now: DateTime<Utc>,
) -> Result<bool> {
    if pressure_pct < AUTO_COMPACT_THRESHOLD {
        return Ok(false);
    }
    // Don't overwrite an existing request (operator takes precedence).
    if is_pending(data_dir) {
        return Ok(false);
    }
    let req = CompactionRequest::automatic(pressure_pct, goal.map(ToString::to_string), now);
    request_compact(data_dir, &req)?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;
    use tempfile::tempdir;

    use super::*;

    fn now() -> DateTime<Utc> {
        chrono::Utc.with_ymd_and_hms(2026, 4, 14, 9, 0, 0).unwrap()
    }

    #[test]
    fn operator_compact_round_trips() {
        let tmp = tempdir().unwrap();
        let req = CompactionRequest::operator(Some("G-042".to_string()), now());
        request_compact(tmp.path(), &req).unwrap();
        assert!(is_pending(tmp.path()));

        let loaded = consume_compact(tmp.path()).unwrap().unwrap();
        assert_eq!(loaded.trigger, CompactionTrigger::Operator);
        assert_eq!(loaded.goal.as_deref(), Some("G-042"));

        assert!(!is_pending(tmp.path()));
    }

    #[test]
    fn consume_returns_none_when_absent() {
        let tmp = tempdir().unwrap();
        assert!(consume_compact(tmp.path()).unwrap().is_none());
    }

    #[test]
    fn auto_compact_triggered_at_threshold() {
        let tmp = tempdir().unwrap();
        let wrote = compact_if_needed(tmp.path(), 0.85, Some("G-001"), now()).unwrap();
        assert!(wrote);

        let loaded = consume_compact(tmp.path()).unwrap().unwrap();
        assert_eq!(loaded.trigger, CompactionTrigger::Automatic);
        assert!((loaded.pressure_pct.unwrap() - 0.85).abs() < f32::EPSILON);
    }

    #[test]
    fn auto_compact_skipped_below_threshold() {
        let tmp = tempdir().unwrap();
        let wrote = compact_if_needed(tmp.path(), 0.70, None, now()).unwrap();
        assert!(!wrote);
        assert!(!is_pending(tmp.path()));
    }

    #[test]
    fn auto_compact_does_not_overwrite_operator_request() {
        let tmp = tempdir().unwrap();
        let req = CompactionRequest::operator(None, now());
        request_compact(tmp.path(), &req).unwrap();

        let wrote = compact_if_needed(tmp.path(), 0.90, None, now()).unwrap();
        assert!(!wrote);

        // The operator request is still there.
        let loaded = consume_compact(tmp.path()).unwrap().unwrap();
        assert_eq!(loaded.trigger, CompactionTrigger::Operator);
    }
}
