//! Edit checkpoints — Auto-snapshot files before agent modifies them.
//!
//! Moltis has automatic checkpointing before file edits.
//! Provides rollback capability if changes are problematic.

use anyhow::Result;
use std::path::PathBuf;

use crate::paths::PraxisPaths;

/// Checkpoint manager for file edits.
pub struct CheckpointManager {
    snapshots_dir: PathBuf,
}

impl CheckpointManager {
    pub fn new(paths: &PraxisPaths) -> Self {
        let snapshots_dir = paths.data_dir.join("checkpoints");
        Self { snapshots_dir }
    }

    /// Create a checkpoint before editing a file.
    pub fn checkpoint(&self, path: &PathBuf) -> Result<Checkpoint> {
        let path_str = path.to_string_lossy();
        let checksum = self.compute_checksum(path)?;
        
        let checkpoint = Checkpoint {
            original_path: path.clone(),
            checksum,
            timestamp: chrono::Utc::now(),
        };
        
        // Store the checkpoint
        let checkpoint_path = self.snapshot_path(&checkpoint);
        std::fs::create_dir_all(checkpoint_path.parent().unwrap())?;
        std::fs::copy(path, &checkpoint_path)?;
        
        Ok(checkpoint)
    }

    /// Restore a file from checkpoint.
    pub fn restore(&self, checkpoint: &Checkpoint) -> Result<()> {
        let checkpoint_path = self.snapshot_path(checkpoint);
        if checkpoint_path.exists() {
            std::fs::copy(&checkpoint_path, &checkpoint.original_path)?;
        }
        Ok(())
    }

    fn compute_checksum(&self, path: &PathBuf) -> Result<String> {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        let contents = std::fs::read(path)?;
        hasher.update(&contents);
        Ok(format!("{:x}", hasher.finalize()))
    }

    fn snapshot_path(&self, checkpoint: &Checkpoint) -> PathBuf {
        let path_hash = &checkpoint.checksum[..8.min(checkpoint.checksum.len())];
        self.snapshots_dir.join(format!(
            "{}_{}",
            checkpoint.timestamp.format("%Y%m%d_%H%M%S"),
            path_hash
        ))
    }
}

/// A file checkpoint.
#[derive(Debug, Clone)]
pub struct Checkpoint {
    pub original_path: PathBuf,
    pub checksum: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Auto-checkpoint wrapper for file operations.
pub async fn with_checkpoint<F>(
    paths: &PraxisPaths,
    path: &PathBuf,
    f: F,
) -> Result<()>
where
    F: FnOnce() -> Result<()>,
{
    let manager = CheckpointManager::new(paths);
    let checkpoint = manager.checkpoint(path)?;
    
    match f() {
        Ok(()) => Ok(()),
        Err(e) => {
            // Auto-restore on error
            let _ = manager.restore(&checkpoint);
            Err(e)
        }
    }
}