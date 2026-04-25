use std::{fs, path::Path};

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{
    paths::PraxisPaths,
    storage::{AnatomyStore, SqliteSessionStore},
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewAnatomyEntry {
    pub path: String,
    pub description: String,
    pub token_estimate: i64,
    pub last_modified_at: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoredAnatomyEntry {
    pub path: String,
    pub description: String,
    pub token_estimate: i64,
    pub last_modified_at: String,
    pub tags: Vec<String>,
}

pub fn build_entry(path: &Path, content: &str, tags: Vec<String>) -> Result<NewAnatomyEntry> {
    Ok(NewAnatomyEntry {
        path: path.display().to_string(),
        description: describe(path, content),
        token_estimate: estimate_tokens(content),
        last_modified_at: modified_at(path)?,
        tags,
    })
}

pub fn render_summary(path: &str, description: &str, token_estimate: i64) -> String {
    format!("{path}: {description} (~{token_estimate} tokens)")
}

fn describe(path: &Path, content: &str) -> String {
    let first_line = content.lines().map(str::trim).find(|line| !line.is_empty()).unwrap_or("");
    if let Some(title) = first_line.strip_prefix("# ") {
        return title.trim().to_string();
    }
    if !first_line.is_empty() {
        return first_line.chars().take(80).collect::<String>();
    }

    path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("Unknown file")
        .to_string()
}

fn modified_at(path: &Path) -> Result<String> {
    let modified = fs::metadata(path)
        .with_context(|| format!("failed to inspect {}", path.display()))?
        .modified()
        .with_context(|| format!("failed to read modified time for {}", path.display()))?;
    Ok(DateTime::<Utc>::from(modified).to_rfc3339())
}

fn estimate_tokens(content: &str) -> i64 {
    let chars = content.chars().count();
    if chars == 0 { 0 } else { chars.div_ceil(4) as i64 }
}

/// Scan identity and tool files for changes since their last indexed
/// `last_modified_at` and re-index any that have been updated on disk.
///
/// Returns the number of entries that were refreshed.
///
/// This is designed to run during idle / maintenance windows rather than
/// blocking an active session.  It does nothing for files that do not yet
/// exist (they will be indexed on first read through the normal path).
pub fn refresh_stale_anatomy(paths: &PraxisPaths) -> Result<usize> {
    let store = SqliteSessionStore::new(paths.database_file.clone());
    let mut refreshed = 0usize;

    // Build the candidate set: identity files + any .md/.toml files in tools/
    let mut candidates: Vec<std::path::PathBuf> = paths.identity_files();

    if paths.tools_dir.is_dir()
        && let Ok(entries) = fs::read_dir(&paths.tools_dir)
    {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.extension().is_some_and(|ext| ext == "toml" || ext == "json") {
                candidates.push(p);
            }
        }
    }

    for candidate in &candidates {
        if !candidate.exists() {
            continue;
        }
        let disk_modified = match modified_at(candidate) {
            Ok(ts) => ts,
            Err(_) => continue,
        };

        // Check if the stored entry is already up-to-date.
        let needs_refresh = match store.anatomy_last_modified(candidate) {
            Ok(Some(stored)) => stored != disk_modified,
            Ok(None) => true, // never indexed
            Err(_) => true,   // index unreadable — refresh anyway
        };

        if !needs_refresh {
            continue;
        }

        let content = match fs::read_to_string(candidate) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let entry = build_entry(candidate, &content, Vec::new())?;
        if let Err(e) = store.upsert_anatomy_entry(&entry) {
            log::warn!("anatomy refresh: failed to upsert {}: {e}", candidate.display());
            continue;
        }
        refreshed += 1;
    }

    Ok(refreshed)
}
