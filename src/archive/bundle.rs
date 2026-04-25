use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::Connection;

use crate::{
    config::AppConfig,
    identity::{IdentityPolicy, LocalIdentityPolicy},
    paths::PraxisPaths,
    storage::{SessionStore, SqliteSessionStore},
    tools::{FileToolRegistry, ToolRegistry},
};

use super::{
    DATA_DIR_NAME, DATABASE_EXPORT_NAME, EXPORT_FORMAT_VERSION, ExportManifest, MANIFEST_FILE,
    RUNTIME_DIR_NAME, STATE_EXPORT_NAME, tree,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BundleExportSummary {
    pub output_dir: PathBuf,
    pub data_file_count: usize,
    pub schema_version: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BundleImportSummary {
    pub data_dir: PathBuf,
    pub restored_file_count: usize,
    pub schema_version: i64,
}

pub fn export_bundle(
    config: &AppConfig,
    paths: &PraxisPaths,
    output_dir: &Path,
    overwrite: bool,
    now: DateTime<Utc>,
) -> Result<BundleExportSummary> {
    tree::ensure_no_overlap(output_dir, &paths.data_dir, "export output")?;
    tree::prepare_fresh_dir(output_dir, overwrite)?;
    let data_root = output_dir.join(DATA_DIR_NAME);
    let runtime_root = output_dir.join(RUNTIME_DIR_NAME);
    fs::create_dir_all(&data_root)
        .with_context(|| format!("failed to create {}", data_root.display()))?;
    fs::create_dir_all(&runtime_root)
        .with_context(|| format!("failed to create {}", runtime_root.display()))?;

    let data_files = tree::copy_dir_tree(
        &paths.data_dir,
        &data_root,
        &[paths.database_file.clone(), paths.state_file.clone()],
    )?;
    tree::copy_file(&paths.database_file, &runtime_root.join(DATABASE_EXPORT_NAME))?;
    let state_present = paths.state_file.exists();
    if state_present {
        tree::copy_file(&paths.state_file, &runtime_root.join(STATE_EXPORT_NAME))?;
    }

    let manifest = ExportManifest {
        format_version: EXPORT_FORMAT_VERSION,
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
        serde_json::to_string_pretty(&manifest).context("failed to serialize export manifest")?,
    )
    .with_context(|| format!("failed to write {}", output_dir.join(MANIFEST_FILE).display()))?;

    Ok(BundleExportSummary {
        output_dir: output_dir.to_path_buf(),
        data_file_count: manifest.data_files.len(),
        schema_version: manifest.schema_version,
    })
}

pub fn import_bundle(
    data_dir: &Path,
    input_dir: &Path,
    overwrite: bool,
) -> Result<BundleImportSummary> {
    tree::ensure_no_overlap(input_dir, data_dir, "import input")?;
    let manifest_path = input_dir.join(MANIFEST_FILE);
    let manifest: ExportManifest = serde_json::from_str(
        &fs::read_to_string(&manifest_path)
            .with_context(|| format!("failed to read {}", manifest_path.display()))?,
    )
    .with_context(|| format!("failed to parse {}", manifest_path.display()))?;

    tree::prepare_fresh_dir(data_dir, overwrite)?;
    let data_root = input_dir.join(DATA_DIR_NAME);
    let runtime_root = input_dir.join(RUNTIME_DIR_NAME);
    let restored = tree::restore_dir_tree(&data_root, data_dir, &manifest.data_files)?;

    let config_path = data_dir.join("praxis.toml");
    let mut config = AppConfig::load(&config_path)?;
    let db_relative = rebase_runtime_path(
        &manifest.database_path,
        &manifest.source_data_dir,
        DATABASE_EXPORT_NAME,
    );
    let state_relative =
        rebase_runtime_path(&manifest.state_file, &manifest.source_data_dir, STATE_EXPORT_NAME);

    config.instance.data_dir = data_dir.to_path_buf();
    config.database.path = db_relative.clone();
    config.runtime.state_file = state_relative.clone();
    config.save(&config_path)?;

    tree::copy_file(&runtime_root.join(DATABASE_EXPORT_NAME), &data_dir.join(&db_relative))?;
    if manifest.state_present {
        tree::copy_file(&runtime_root.join(STATE_EXPORT_NAME), &data_dir.join(&state_relative))?;
    }

    let config = AppConfig::load(&config_path)?.with_overridden_data_dir(data_dir.to_path_buf());
    let paths = PraxisPaths::from_config(&config);
    let store = SqliteSessionStore::new(paths.database_file.clone());
    store.validate_schema()?;
    LocalIdentityPolicy.validate(&paths)?;
    FileToolRegistry.validate(&paths)?;

    Ok(BundleImportSummary {
        data_dir: data_dir.to_path_buf(),
        restored_file_count: restored + 1 + usize::from(manifest.state_present),
        schema_version: manifest.schema_version,
    })
}

pub(super) fn schema_version(database_file: &Path) -> Result<i64> {
    let connection = Connection::open(database_file)
        .with_context(|| format!("failed to open {}", database_file.display()))?;
    connection
        .query_row("SELECT COALESCE(MAX(version), 0) FROM schema_migrations", [], |row| row.get(0))
        .context("failed to query schema version")
}

fn rebase_runtime_path(raw: &Path, source_data_dir: &Path, fallback: &str) -> PathBuf {
    if raw.is_absolute() {
        if let Ok(relative) = raw.strip_prefix(source_data_dir)
            && let Ok(relative) = normalize_relative(relative)
        {
            return relative;
        }
        return raw.file_name().map(PathBuf::from).unwrap_or_else(|| PathBuf::from(fallback));
    }
    normalize_relative(raw).unwrap_or_else(|_| PathBuf::from(fallback))
}

fn normalize_relative(raw: &Path) -> Result<PathBuf> {
    tree::safe_relative(&raw.to_string_lossy())
}
