use std::{fs, path::Path};

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SessionPhase {
    Orient,
    Decide,
    Act,
    Reflect,
    Sleep,
}

impl std::fmt::Display for SessionPhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            SessionPhase::Orient => "orient",
            SessionPhase::Decide => "decide",
            SessionPhase::Act => "act",
            SessionPhase::Reflect => "reflect",
            SessionPhase::Sleep => "sleep",
        };

        f.write_str(value)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionState {
    pub current_phase: SessionPhase,
    pub started_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub selected_goal_id: Option<String>,
    pub selected_goal_title: Option<String>,
    pub requested_task: Option<String>,
    pub orientation_summary: Option<String>,
    pub action_summary: Option<String>,
    pub last_outcome: Option<String>,
    pub resume_count: u32,
}

impl SessionState {
    pub fn new(now: DateTime<Utc>, requested_task: Option<String>) -> Self {
        Self {
            current_phase: SessionPhase::Orient,
            started_at: now,
            updated_at: now,
            completed_at: None,
            selected_goal_id: None,
            selected_goal_title: None,
            requested_task,
            orientation_summary: None,
            action_summary: None,
            last_outcome: None,
            resume_count: 0,
        }
    }

    pub fn load(path: &Path) -> Result<Option<Self>> {
        if !path.exists() {
            return Ok(None);
        }

        let raw = fs::read_to_string(path)
            .with_context(|| format!("failed to read session state {}", path.display()))?;
        let state = serde_json::from_str(&raw)
            .with_context(|| format!("invalid session state in {}", path.display()))?;
        Ok(Some(state))
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        let raw =
            serde_json::to_string_pretty(self).context("failed to serialize session state")?;
        fs::write(path, raw).with_context(|| format!("failed to write {}", path.display()))?;
        Ok(())
    }

    pub fn is_incomplete(&self) -> bool {
        self.completed_at.is_none() && self.current_phase != SessionPhase::Sleep
    }

    pub fn mark_phase(&mut self, phase: SessionPhase, now: DateTime<Utc>) {
        self.current_phase = phase;
        self.updated_at = now;
    }

    pub fn finish(&mut self, outcome: impl Into<String>, now: DateTime<Utc>) {
        self.current_phase = SessionPhase::Sleep;
        self.last_outcome = Some(outcome.into());
        self.completed_at = Some(now);
        self.updated_at = now;
    }
}
