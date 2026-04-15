use std::{fs, path::PathBuf};

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};

use crate::{config::AppConfig, paths::PraxisPaths};

use super::{
    DATA_DIR_NAME, DATABASE_EXPORT_NAME, ExportManifest, MANIFEST_FILE, RUNTIME_DIR_NAME,
    STATE_EXPORT_NAME, bundle::schema_version, tree,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotExportSummary {
    pub output_dir: PathBuf,
    pub data_file_count: usize,
    pub schema_version: i64,
    pub pruned: usize,
}

pub fn maybe_create_daily_snapshot(
    config: &AppConfig,
    paths: &PraxisPaths,
    now: DateTime<Utc>,
) -> Result<Option<SnapshotExportSummary>> {
    let output_dir = paths
        .backups_dir
        .join(format!("snapshot-{}", now.format("%Y%m%d")));
    if output_dir.exists() {
        return Ok(None);
    }

    let summary = export_snapshot(config, paths, &output_dir, now)?;
    let pruned = prune_old_snapshots(&paths.backups_dir, config.runtime.snapshot_retention_days)?;
    Ok(Some(SnapshotExportSummary {
        output_dir: summary.output_dir,
        data_file_count: summary.data_file_count,
        schema_version: summary.schema_version,
        pruned,
    }))
}

fn export_snapshot(
    config: &AppConfig,
    paths: &PraxisPaths,
    output_dir: &std::path::Path,
    now: DateTime<Utc>,
) -> Result<SnapshotExportSummary> {
    tree::prepare_fresh_dir(output_dir, false)?;
    let data_root = output_dir.join(DATA_DIR_NAME);
    let runtime_root = output_dir.join(RUNTIME_DIR_NAME);
    fs::create_dir_all(&data_root)
        .with_context(|| format!("failed to create {}", data_root.display()))?;
    fs::create_dir_all(&runtime_root)
        .with_context(|| format!("failed to create {}", runtime_root.display()))?;

    let data_files = tree::copy_dir_tree(
        &paths.data_dir,
        &data_root,
        &[
            paths.database_file.clone(),
            paths.state_file.clone(),
            paths.backups_dir.clone(),
        ],
    )?;
    tree::copy_file(
        &paths.database_file,
        &runtime_root.join(DATABASE_EXPORT_NAME),
    )?;
    let state_present = paths.state_file.exists();
    if state_present {
        tree::copy_file(&paths.state_file, &runtime_root.join(STATE_EXPORT_NAME))?;
    }

    let manifest = ExportManifest {
        format_version: super::EXPORT_FORMAT_VERSION,
        exported_at: now.to_rfc3339(),
        source_data_dir: paths.data_dir.clone(),
        database_path: config.database.path.clone(),
        state_file: config.runtime.state_file.clone(),
        schema_version: schema_version(&paths.database_file)?,
        state_present,
        data_files,
    };
    fs::write(
        output_dir.join(MANIFEST_FILE),
        serde_json::to_string_pretty(&manifest).context("failed to serialize snapshot manifest")?,
    )
    .with_context(|| {
        format!(
            "failed to write {}",
            output_dir.join(MANIFEST_FILE).display()
        )
    })?;

    Ok(SnapshotExportSummary {
        output_dir: output_dir.to_path_buf(),
        data_file_count: manifest.data_files.len(),
        schema_version: manifest.schema_version,
        pruned: 0,
    })
}

fn prune_old_snapshots(root: &std::path::Path, keep: usize) -> Result<usize> {
    if !root.exists() {
        return Ok(0);
    }

    let mut snapshots = fs::read_dir(root)?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect::<Vec<_>>();
    snapshots.sort();

    let remove_count = snapshots.len().saturating_sub(keep);
    for path in snapshots.into_iter().take(remove_count) {
        fs::remove_dir_all(&path)
            .with_context(|| format!("failed to remove {}", path.display()))?;
    }
    Ok(remove_count)
}
