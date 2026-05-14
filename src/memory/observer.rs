//! Session observation tracker.
//!
//! Records which sessions have had memory extraction run on them,
//! preventing redundant processing. Works hand-in-hand with `capture.rs`.
//!
//! The observer is NOT a background cron. The reflect phase calls
//! `observe_session()` after capture extraction completes, which
//! persists durable observations as additional hot memories.
//!
//! Checkpoint is stored as a simple JSON file for lightweight tracking
//! without adding DB queries to the critical path.

use std::path::Path;

use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::{
    backend::AgentBackend,
    memory::{MemoryStore, NewHotMemory},
};

/// Checkpoint tracking extraction progress.
#[derive(Debug, Serialize, Deserialize)]
pub struct ObservationCheckpoint {
    /// Highest session ID that had memory extraction run.
    pub last_session_id: i64,
    /// When the last observation ran.
    pub last_observed_at: String,
}

/// Stats from an observation pass.
#[derive(Debug, Default)]
pub struct ObservationSummary {
    pub sessions_processed: usize,
    pub memories_extracted: usize,
}

impl ObservationCheckpoint {
    /// Load from disk or create a fresh one.
    pub fn load_or_fresh(path: &Path) -> Self {
        if path.exists() {
            std::fs::read_to_string(path)
                .ok()
                .and_then(|content| serde_json::from_str(&content).ok())
                .unwrap_or_else(|| {
                    log::warn!("memory observer: corrupted checkpoint, starting fresh");
                    Self::fresh()
                })
        } else {
            Self::fresh()
        }
    }

    /// Persist checkpoint to disk.
    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, &content)?;
        Ok(())
    }

    fn fresh() -> Self {
        Self {
            last_session_id: 0,
            last_observed_at: Utc::now().to_rfc3339(),
        }
    }
}

/// Extract durable observations from a session and persist them as hot memories.
///
/// Called from the reflect phase after `capture::extract()` runs.
/// This is the "second pass" — captures less obvious facts that benefit
/// from having the full session context.
pub fn observe_session<S, B>(
    store: &S,
    backend: &B,
    session: &crate::storage::StoredSession,
    checkpoint_path: &Path,
) -> Result<ObservationSummary>
where
    S: MemoryStore,
    B: AgentBackend,
{
    // Skip non-sessions
    if session.action_summary.is_empty()
        || matches!(session.outcome.as_str(), "idle" | "skipped" | "deferred")
    {
        return Ok(ObservationSummary::default());
    }

    let prompt = format!(
        "You are a memory extraction assistant. Given a completed agent session, \
         extract observable facts worth remembering as a JSON array.

Each observation:
- \"content\": the fact (one sentence, specific)
- \"summary\": one-line summary (optional)
- \"importance\": 0.0-1.0 (0.5+ worth keeping)
- \"tags\": [\"category\"] (preference, pattern, decision, discovery, gotcha)
- \"memory_type\": \"episodic\" or \"semantic\"

Focus on: user preferences, system quirks, recurring patterns, decisions, 
architecture insights, lessons learned. Exclude trivial status updates.

Session outcome: {}
Session summary: {}
Goal: {}

Return ONLY valid JSON:",
        session.outcome,
        session.action_summary,
        session.selected_goal_title.as_deref().unwrap_or("none"),
    );

    let result = match backend.answer_prompt(&prompt) {
        Ok(output) => output.summary,
        Err(e) => {
            log::warn!("memory observer: LLM extraction failed: {e}");
            return Ok(ObservationSummary::default());
        }
    };

    let observations = parse_observations(&result);
    let mut summary = ObservationSummary::default();

    for obs in &observations {
        if obs.importance < 0.5 || obs.content.trim().is_empty() {
            continue;
        }

        let tags = normalize_tags(&obs.tags, &session.outcome);
        let memory_type = crate::memory::MemoryType::parse(&obs.memory_type);

        if let Err(e) = store.insert_hot_memory(NewHotMemory {
            content: format!("[obs] {}", obs.content),
            summary: obs.summary.clone(),
            importance: obs.importance.min(0.85),
            tags,
            expires_at: None,
            memory_type,
        }) {
            log::warn!("memory observer: insert failed: {e}");
            continue;
        }
        summary.memories_extracted += 1;
    }

    summary.sessions_processed = 1;

    // Update checkpoint
    let checkpoint = ObservationCheckpoint {
        last_session_id: session.id,
        last_observed_at: Utc::now().to_rfc3339(),
    };
    if let Err(e) = checkpoint.save(checkpoint_path) {
        log::warn!("memory observer: failed to save checkpoint: {e}");
    }

    log::info!(
        "memory observer: session {} — extracted {} observations",
        session.id,
        summary.memories_extracted,
    );

    Ok(summary)
}

#[derive(Debug, Deserialize)]
struct Observation {
    content: String,
    summary: Option<String>,
    importance: f32,
    tags: Vec<String>,
    #[serde(default = "default_memory_type")]
    memory_type: String,
}

fn default_memory_type() -> String {
    "episodic".to_string()
}

fn parse_observations(raw: &str) -> Vec<Observation> {
    let trimmed = raw.trim();

    let json_str = if trimmed.starts_with("```") {
        let s = trimmed
            .strip_prefix("```json")
            .or_else(|| trimmed.strip_prefix("```"))
            .unwrap_or(trimmed);
        s.strip_suffix("```").unwrap_or(s).trim()
    } else {
        trimmed
    };

    match serde_json::from_str::<Vec<Observation>>(json_str) {
        Ok(obs) => obs,
        Err(_) => match serde_json::from_str::<Observation>(json_str) {
            Ok(single) => vec![single],
            Err(_) => {
                log::warn!("memory observer: failed to parse LLM response");
                Vec::new()
            }
        },
    }
}

fn normalize_tags(tags: &[String], outcome: &str) -> Vec<String> {
    let mut normalized: Vec<String> =
        tags.iter().map(|t| t.to_lowercase().replace(' ', "-")).collect();
    normalized.push("observation".to_string());
    normalized.push(format!("outcome:{outcome}"));
    normalized.sort();
    normalized.dedup();
    normalized
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_empty() {
        assert!(parse_observations("[]").is_empty());
    }

    #[test]
    fn test_parse_single() {
        let r = parse_observations(
            r#"{"content":"test","importance":0.7,"tags":["a"],"memory_type":"semantic"}"#,
        );
        assert_eq!(r.len(), 1);
    }

    #[test]
    fn test_checkpoint_roundtrip() {
        let dir = std::env::temp_dir().join("praxis_observer_test");
        let path = dir.join("ckpt.json");
        let cp = ObservationCheckpoint {
            last_session_id: 42,
            last_observed_at: "2026-01-01T00:00:00Z".to_string(),
        };
        cp.save(&path).unwrap();
        let loaded = ObservationCheckpoint::load_or_fresh(&path);
        assert_eq!(loaded.last_session_id, 42);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
