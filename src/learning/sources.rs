use std::{collections::BTreeMap, fs};

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};

use crate::{
    learning::{NewLearningSourceState, entries},
    paths::PraxisPaths,
    storage::SqliteSessionStore,
};

pub(super) struct SourceProcessing {
    pub processed_sources: usize,
    pub changed_sources: usize,
    pub notes: Vec<String>,
}

pub(super) fn process_sources(
    paths: &PraxisPaths,
    store: &SqliteSessionStore,
    now: DateTime<Utc>,
) -> Result<SourceProcessing> {
    let known = store
        .list_learning_sources()?
        .into_iter()
        .map(|source| (source.path.clone(), source))
        .collect::<BTreeMap<_, _>>();
    let mut processed_sources = 0;
    let mut changed_sources = 0;
    let mut notes = Vec::new();

    for path in source_files(paths)? {
        processed_sources += 1;
        let metadata = fs::metadata(&path)
            .with_context(|| format!("failed to read metadata for {}", path.display()))?;
        let last_modified_at = DateTime::<Utc>::from(
            metadata
                .modified()
                .with_context(|| format!("failed to inspect {}", path.display()))?,
        )
        .to_rfc3339();
        let byte_len = metadata.len() as i64;
        let relative = path
            .strip_prefix(&paths.data_dir)
            .unwrap_or(&path)
            .display()
            .to_string();

        if known.get(&relative).is_some_and(|existing| {
            existing.last_modified_at == last_modified_at && existing.byte_len == byte_len
        }) {
            continue;
        }

        let raw = fs::read_to_string(&path)
            .with_context(|| format!("failed to read learning source {}", path.display()))?;
        let summary = summarize_source(&raw);
        entries::append_source_learning(paths, &relative, &summary, now)?;
        store.upsert_learning_source(NewLearningSourceState {
            path: relative.clone(),
            last_modified_at,
            byte_len,
            summary,
            last_processed_at: now,
        })?;
        changed_sources += 1;
        notes.push(format!("Captured fresh learning from {relative}."));
    }

    Ok(SourceProcessing {
        processed_sources,
        changed_sources,
        notes,
    })
}

fn source_files(paths: &PraxisPaths) -> Result<Vec<std::path::PathBuf>> {
    let mut files = Vec::new();
    if !paths.learning_sources_dir.exists() {
        return Ok(files);
    }

    for entry in fs::read_dir(&paths.learning_sources_dir)
        .with_context(|| format!("failed to read {}", paths.learning_sources_dir.display()))?
    {
        let path = entry?.path();
        let extension = path.extension().and_then(|value| value.to_str());
        if matches!(extension, Some("md") | Some("txt")) {
            files.push(path);
        }
    }

    files.sort();
    Ok(files)
}

fn summarize_source(raw: &str) -> String {
    let lines = raw
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .take(3)
        .map(|line| line.trim_start_matches(['#', '-', '*', ' ']))
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();

    if lines.is_empty() {
        "Updated learning source with no extractable summary.".to_string()
    } else {
        lines.join("; ")
    }
}
