mod audit;
mod bundle;
mod snapshots;
mod tree;

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

pub use audit::{AuditExportSummary, export_audit};
pub use bundle::{BundleExportSummary, BundleImportSummary, export_bundle, import_bundle};
pub use snapshots::{SnapshotExportSummary, maybe_create_daily_snapshot};

pub(crate) const EXPORT_FORMAT_VERSION: u32 = 1;
pub(crate) const MANIFEST_FILE: &str = "manifest.json";
pub(crate) const DATA_DIR_NAME: &str = "data";
pub(crate) const RUNTIME_DIR_NAME: &str = "runtime";
pub(crate) const DATABASE_EXPORT_NAME: &str = "database.sqlite";
pub(crate) const STATE_EXPORT_NAME: &str = "session_state.json";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ExportManifest {
    pub format_version: u32,
    pub exported_at: String,
    pub source_data_dir: PathBuf,
    pub database_path: PathBuf,
    pub state_file: PathBuf,
    pub schema_version: i64,
    pub state_present: bool,
    pub data_files: Vec<String>,
}
