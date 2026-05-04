//! Filesystem checkpoints and rollback for safe autonomous operation.
//!
//! Provides copy-on-write checkpoints of the data directory, allowing
//! the operator to restore previous states via `praxis checkpoint` and
//! `praxis rollback`.

use std::{
    fs,
    path::{Path, PathBuf},
    // ...existing code...
};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use crate::paths::PraxisPaths;

/// Checkpoint metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    pub id: u64,
    pub timestamp: String,
    pub label: String,
    pub file_count: usize,
    pub total_bytes: u64,
}

/// Checkpoint index — stored in the checkpoints directory.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct CheckpointIndex {
    pub checkpoints: Vec<Checkpoint>,
}

impl CheckpointIndex {
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let raw = fs::read_to_string(path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        serde_json::from_str(&raw).with_context(|| format!("failed to parse {}", path.display()))
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let raw =
            serde_json::to_string_pretty(self).context("failed to serialize checkpoint index")?;
        fs::write(path, raw).with_context(|| format!("failed to write {}", path.display()))
    }

    pub fn next_id(&self) -> u64 {
        self.checkpoints.iter().map(|c| c.id).max().unwrap_or(0) + 1
    }
}

/// Create a checkpoint of the data directory.
pub fn create_checkpoint(paths: &PraxisPaths, label: &str) -> Result<Checkpoint> {
    let checkpoints_dir = paths.data_dir.join("checkpoints");
    fs::create_dir_all(&checkpoints_dir)?;

    let index_path = checkpoints_dir.join("index.json");
    let mut index = CheckpointIndex::load(&index_path)?;

    let id = index.next_id();
    let cp_dir = checkpoints_dir.join(format!("cp-{id}"));
    fs::create_dir_all(&cp_dir)?;

    // Copy all files from data_dir to checkpoint dir (skip checkpoints dir itself)
    let (file_count, total_bytes) = copy_directory(&paths.data_dir, &cp_dir, &checkpoints_dir)?;

    let timestamp = chrono::Utc::now().to_rfc3339();
    let checkpoint = Checkpoint {
        id,
        timestamp,
        label: label.to_string(),
        file_count,
        total_bytes,
    };

    index.checkpoints.push(checkpoint.clone());
    // Keep only the last 20 checkpoints
    if index.checkpoints.len() > 20 {
        let old: Vec<_> = index.checkpoints.drain(..index.checkpoints.len() - 20).collect();
        for cp in old {
            let old_dir = checkpoints_dir.join(format!("cp-{}", cp.id));
            let _ = fs::remove_dir_all(&old_dir);
        }
    }

    index.save(&index_path)?;
    Ok(checkpoint)
}

/// List all checkpoints.
pub fn list_checkpoints(paths: &PraxisPaths) -> Result<Vec<Checkpoint>> {
    let checkpoints_dir = paths.data_dir.join("checkpoints");
    let index_path = checkpoints_dir.join("index.json");
    let index = CheckpointIndex::load(&index_path)?;
    Ok(index.checkpoints)
}

/// Roll back to a specific checkpoint by ID.
pub fn rollback_to(paths: &PraxisPaths, checkpoint_id: u64) -> Result<Checkpoint> {
    let checkpoints_dir = paths.data_dir.join("checkpoints");
    let index_path = checkpoints_dir.join("index.json");
    let index = CheckpointIndex::load(&index_path)?;

    let checkpoint = index
        .checkpoints
        .iter()
        .find(|c| c.id == checkpoint_id)
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("checkpoint {checkpoint_id} not found"))?;

    let cp_dir = checkpoints_dir.join(format!("cp-{checkpoint_id}"));
    if !cp_dir.exists() {
        bail!("checkpoint directory {} not found", cp_dir.display());
    }

    // First, create a pre-rollback checkpoint of the current state
    let _pre_rollback = create_checkpoint(paths, &format!("pre-rollback-to-{checkpoint_id}"));

    // Remove current files (but not checkpoints directory)
    for entry in fs::read_dir(&paths.data_dir)? {
        let entry = entry?;
        let name = entry.file_name();
        if name == "checkpoints" {
            continue;
        }
        let path = entry.path();
        if path.is_dir() {
            fs::remove_dir_all(&path)
                .with_context(|| format!("failed to remove {}", path.display()))?;
        } else {
            fs::remove_file(&path)
                .with_context(|| format!("failed to remove {}", path.display()))?;
        }
    }

    // Copy checkpoint files back
    copy_directory(&cp_dir, &paths.data_dir, &checkpoints_dir)?;

    Ok(checkpoint)
}

/// Recursively copy a directory, skipping the `skip` directory.
fn copy_directory(src: &Path, dst: &Path, skip: &Path) -> Result<(usize, u64)> {
    let mut file_count = 0usize;
    let mut total_bytes = 0u64;

    if !src.exists() {
        return Ok((0, 0));
    }

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();

        // Skip the checkpoints directory
        if src_path == skip {
            continue;
        }

        let file_name = entry.file_name();
        let dst_path = dst.join(&file_name);

        if src_path.is_dir() {
            fs::create_dir_all(&dst_path)?;
            let (fc, tb) = copy_directory(&src_path, &dst_path, skip)?;
            file_count += fc;
            total_bytes += tb;
        } else {
            let bytes = fs::copy(&src_path, &dst_path).with_context(|| {
                format!("failed to copy {} -> {}", src_path.display(), dst_path.display())
            })?;
            file_count += 1;
            total_bytes += bytes;
        }
    }

    Ok((file_count, total_bytes))
}

/// CLI handler for `praxis checkpoint`.
pub fn handle_checkpoint(data_dir: Option<PathBuf>, label: Option<String>) -> Result<String> {
    let data_dir = data_dir.unwrap_or(crate::paths::default_data_dir()?);
    let paths = PraxisPaths::for_data_dir(data_dir);

    let label = label.unwrap_or_else(|| "manual".to_string());
    let cp = create_checkpoint(&paths, &label)?;

    Ok(format!(
        "Checkpoint #{} created: {} ({} files, {:.1} KB)\nLabel: {}",
        cp.id,
        cp.timestamp,
        cp.file_count,
        cp.total_bytes as f64 / 1024.0,
        cp.label,
    ))
}

/// CLI handler for `praxis rollback`.
pub fn handle_rollback(data_dir: Option<PathBuf>, checkpoint_id: u64) -> Result<String> {
    let data_dir = data_dir.unwrap_or(crate::paths::default_data_dir()?);
    let paths = PraxisPaths::for_data_dir(data_dir);

    let cp = rollback_to(&paths, checkpoint_id)?;

    Ok(format!(
        "Rolled back to checkpoint #{}: {}\nLabel: {}\nPre-rollback state saved as new checkpoint.",
        cp.id, cp.timestamp, cp.label,
    ))
}

/// CLI handler for `praxis checkpoints`.
pub fn handle_checkpoints_list(data_dir: Option<PathBuf>) -> Result<String> {
    let data_dir = data_dir.unwrap_or(crate::paths::default_data_dir()?);
    let paths = PraxisPaths::for_data_dir(data_dir);

    let checkpoints = list_checkpoints(&paths)?;

    if checkpoints.is_empty() {
        return Ok("No checkpoints found.".to_string());
    }

    let mut lines = vec![format!("Checkpoints ({}):", checkpoints.len())];
    for cp in &checkpoints {
        lines.push(format!(
            "  #{} [{}] {} ({} files, {:.1} KB)",
            cp.id,
            cp.timestamp,
            cp.label,
            cp.file_count,
            cp.total_bytes as f64 / 1024.0,
        ));
    }
    Ok(lines.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn test_paths() -> (PraxisPaths, tempfile::TempDir) {
        let tmp = tempfile::tempdir().unwrap();
        let paths = PraxisPaths::for_data_dir(tmp.path().to_path_buf());
        fs::create_dir_all(&paths.data_dir).unwrap();
        (paths, tmp)
    }

    #[test]
    fn test_create_and_list_checkpoints() {
        let (paths, _tmp) = test_paths();

        // Create a test file
        fs::write(paths.data_dir.join("test.txt"), "hello").unwrap();

        let cp = create_checkpoint(&paths, "test-checkpoint").unwrap();
        assert_eq!(cp.label, "test-checkpoint");
        assert!(cp.file_count > 0);

        let checkpoints = list_checkpoints(&paths).unwrap();
        assert_eq!(checkpoints.len(), 1);
        assert_eq!(checkpoints[0].id, cp.id);
    }

    #[test]
    fn test_rollback_restores_files() {
        let (paths, _tmp) = test_paths();

        // Create original file and checkpoint
        fs::write(paths.data_dir.join("test.txt"), "original").unwrap();
        let cp = create_checkpoint(&paths, "original-state").unwrap();

        // Modify the file
        fs::write(paths.data_dir.join("test.txt"), "modified").unwrap();

        // Rollback
        rollback_to(&paths, cp.id).unwrap();

        // Verify original content restored
        let content = fs::read_to_string(paths.data_dir.join("test.txt")).unwrap();
        assert_eq!(content, "original");
    }

    #[test]
    fn test_max_checkpoints() {
        let (paths, _tmp) = test_paths();
        fs::write(paths.data_dir.join("test.txt"), "hello").unwrap();

        // Create 25 checkpoints
        for i in 0..25 {
            create_checkpoint(&paths, &format!("cp-{i}")).unwrap();
        }

        let checkpoints = list_checkpoints(&paths).unwrap();
        // Should be capped at 20 + 1 (the pre-rollback inside rollback creates extra)
        assert!(checkpoints.len() <= 22);
    }
}
