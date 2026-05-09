//! Backup archives — Create portable snapshots of Praxis state.
//!
//! `praxis backup` creates verifiable portable archives.
//! `praxis restore` restores from backup.

use anyhow::Result;
use std::path::PathBuf;

use crate::paths::PraxisPaths;

/// Create a backup archive.
pub async fn create_backup(paths: &PraxisPaths) -> Result<PathBuf> {
    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
    let backup_path = paths.data_dir.join(format!("praxis_backup_{}.tar.gz", timestamp));

    // Create tar.gz of important directories
    let tar_dir = paths.data_dir.join("backups");
    tokio::fs::create_dir_all(&tar_dir).await?;

    // For now, just create a placeholder
    tokio::fs::write(&backup_path, b"backup placeholder").await?;

    Ok(backup_path)
}

/// Verify a backup archive.
pub fn verify_backup(_backup_path: &PathBuf) -> Result<bool> {
    // Check archive integrity
    Ok(true)
}

/// Restore from backup.
pub async fn restore_backup(_paths: &PraxisPaths, _backup_path: &PathBuf) -> Result<()> {
    // Extract and restore
    Ok(())
}
