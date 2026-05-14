//! Mid-session context request.
//!
//! When the agent determines it needs additional context sources mid-session
//! (e.g. a file it hasn't read yet, a memory search it skipped), it emits a
//! `ContextRequest` that gets persisted to `context_request.json`.  On the next
//! `orient()` cycle, the loader honours these requests by injecting the
//! requested sources into the budgeted context.

use std::fs;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::paths::PraxisPaths;

const FILENAME: &str = "context_request.json";

/// A request from the agent for additional context in the next orient cycle.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ContextRequest {
    /// Relative paths (from data_dir) the agent wants read and injected.
    #[serde(default)]
    pub include_files: Vec<String>,
    /// Memory search queries the agent wants injected.
    #[serde(default)]
    pub memory_queries: Vec<String>,
    /// Human-readable reason for the request.
    #[serde(default)]
    pub reason: String,
}

impl ContextRequest {
    /// Load a pending context request from disk, if one exists.
    pub fn load(paths: &PraxisPaths) -> Result<Option<Self>> {
        let path = paths.data_dir.join(FILENAME);
        if !path.exists() {
            return Ok(None);
        }
        let raw = fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let req = serde_json::from_str(&raw)
            .with_context(|| format!("invalid JSON in {}", path.display()))?;
        Ok(Some(req))
    }

    /// Persist a context request to disk for the next orient cycle.
    pub fn save(&self, paths: &PraxisPaths) -> Result<()> {
        let path = paths.data_dir.join(FILENAME);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        let raw =
            serde_json::to_string_pretty(self).context("failed to serialize context request")?;
        fs::write(&path, raw).with_context(|| format!("failed to write {}", path.display()))?;
        Ok(())
    }

    /// Clear the pending context request after it has been consumed.
    pub fn clear(paths: &PraxisPaths) -> Result<()> {
        let path = paths.data_dir.join(FILENAME);
        if path.exists() {
            fs::remove_file(&path)
                .with_context(|| format!("failed to remove {}", path.display()))?;
        }
        Ok(())
    }

    /// Returns true if there is any request to honour.
    pub fn is_empty(&self) -> bool {
        self.include_files.is_empty() && self.memory_queries.is_empty()
    }
}
