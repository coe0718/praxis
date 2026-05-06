//! Session spawn implementation.

use std::collections::HashMap;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::paths::PraxisPaths;
use crate::wakeup::{WakeIntent, WakePriority};

/// Status of a spawned session.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SpawnStatus {
    /// Session intent written, daemon will pick it up.
    Spawned,
    /// Session is actively running.
    Running,
    /// Session failed to spawn.
    Failed,
}

/// Request to spawn a new agent session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSpawnRequest {
    /// Task description injected into the spawned session.
    pub task_description: String,
    /// Parent session ID (for kanban worker tracking).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_session_id: Option<String>,
    /// Environment variables to set in the spawned session.
    #[serde(default)]
    pub environment_vars: HashMap<String, String>,
    /// Working directory for the spawned session.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub working_directory: Option<String>,
    /// Additional metadata.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
    /// Priority — urgent bypasses quiet hours.
    #[serde(default)]
    pub priority: SpawnPriority,
}

impl Default for SpawnPriority {
    fn default() -> Self {
        SpawnPriority::Normal
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SpawnPriority {
    Normal,
    Urgent,
}

/// Result of a session spawn operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSpawnResult {
    /// Unique identifier for this spawn request.
    pub spawn_id: String,
    /// Current status.
    pub status: SpawnStatus,
    /// Kanban task ID if spawned from kanban.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kanban_task_id: Option<String>,
    /// Timestamp of spawn.
    pub spawned_at: i64,
}

/// Spawns new agent sessions by writing WakeIntents.
pub struct SessionSpawner {
    paths: PraxisPaths,
}

impl SessionSpawner {
    pub fn new(paths: &PraxisPaths) -> Self {
        Self { paths: paths.clone() }
    }

    /// Spawn a new session by writing a WakeIntent to the data directory.
    ///
    /// The daemon picks up the intent on its next poll cycle and creates
    /// a new session with the given task description.
    pub fn spawn(&self, req: SessionSpawnRequest) -> Result<SessionSpawnResult> {
        let spawn_id = format!("spawn-{}", now_secs());

        // Build WakeIntent
        let priority = match req.priority {
            SpawnPriority::Urgent => WakePriority::Urgent,
            SpawnPriority::Normal => WakePriority::Normal,
        };

        let source = if req.parent_session_id.is_some()
            || req.environment_vars.contains_key("PRAXIS_KANBAN_TASK")
        {
            "kanban"
        } else {
            "sessions_spawn"
        };

        let wake = WakeIntent {
            reason: format!("session_spawn: {}", req.task_description),
            source: source.to_string(),
            task: Some(req.task_description.clone()),
            priority,
            created_at: Utc::now(),
        };

        // Write WakeIntent
        let wake_path = self.paths.data_dir.join("wake_intent.json");
        let json = serde_json::to_string_pretty(&wake).context("serialize wake intent")?;
        fs::write(&wake_path, json).with_context(|| format!("write {}", wake_path.display()))?;

        // Persist spawn record for tracking
        let record = SessionSpawnResult {
            spawn_id: spawn_id.clone(),
            status: SpawnStatus::Spawned,
            kanban_task_id: req.environment_vars.get("PRAXIS_KANBAN_TASK").cloned(),
            spawned_at: now_secs(),
        };

        self.save_spawn_record(&record)?;

        // Persist environment vars for the daemon to pick up
        if !req.environment_vars.is_empty() {
            let env_path = self.paths.data_dir.join("spawn_env.json");
            let env_json = serde_json::to_string_pretty(&req.environment_vars)
                .context("serialize spawn env vars")?;
            fs::write(&env_path, env_json)
                .with_context(|| format!("write {}", env_path.display()))?;
        }

        log::info!(
            "session spawn: {} — task: {}",
            spawn_id,
            &req.task_description[..req.task_description.len().min(80)]
        );

        Ok(record)
    }

    /// List all spawned session records.
    pub fn list_spawned(&self) -> Result<Vec<SessionSpawnResult>> {
        let spawns_dir = self.paths.data_dir.join("spawns");
        if !spawns_dir.exists() {
            return Ok(Vec::new());
        }

        let mut records = Vec::new();
        for entry in
            fs::read_dir(&spawns_dir).with_context(|| format!("read {}", spawns_dir.display()))?
        {
            let entry = entry?;
            if entry.path().extension().map(|e| e == "json").unwrap_or(false) {
                let raw = fs::read_to_string(entry.path())?;
                if let Ok(record) = serde_json::from_str::<SessionSpawnResult>(&raw) {
                    records.push(record);
                }
            }
        }

        records.sort_by(|a, b| b.spawned_at.cmp(&a.spawned_at));
        Ok(records)
    }

    /// Check the status of a spawned session.
    pub fn check_status(&self, spawn_id: &str) -> Result<Option<SessionSpawnResult>> {
        let path = self.paths.data_dir.join("spawns").join(format!("{}.json", spawn_id));
        if !path.exists() {
            return Ok(None);
        }
        let raw = fs::read_to_string(&path)?;
        let record: SessionSpawnResult =
            serde_json::from_str(&raw).with_context(|| format!("parse {}", path.display()))?;
        Ok(Some(record))
    }

    fn save_spawn_record(&self, record: &SessionSpawnResult) -> Result<()> {
        let spawns_dir = self.paths.data_dir.join("spawns");
        fs::create_dir_all(&spawns_dir)
            .with_context(|| format!("create {}", spawns_dir.display()))?;

        let path = spawns_dir.join(format!("{}.json", record.spawn_id));
        let json = serde_json::to_string_pretty(record).context("serialize spawn record")?;
        fs::write(&path, json).with_context(|| format!("write {}", path.display()))?;
        Ok(())
    }
}

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn spawn_request_serialization() {
        let req = SessionSpawnRequest {
            task_description: "Test task".to_string(),
            parent_session_id: Some("parent-123".to_string()),
            environment_vars: HashMap::new(),
            working_directory: None,
            metadata: None,
            priority: SpawnPriority::Normal,
        };
        let json = serde_json::to_string(&req).unwrap();
        let parsed: SessionSpawnRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.task_description, "Test task");
    }
}
