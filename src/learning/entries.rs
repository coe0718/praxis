use std::{fs, io::Write};

use anyhow::{Context, Result, bail};
use chrono::{DateTime, Utc};

use crate::paths::PraxisPaths;

pub(super) fn append_source_learning(
    paths: &PraxisPaths,
    source: &str,
    summary: &str,
    now: DateTime<Utc>,
) -> Result<()> {
    append_entry(paths, "source-summary", source, summary, now)
}

pub(super) fn append_operational_learning(
    paths: &PraxisPaths,
    summary: &str,
    now: DateTime<Utc>,
) -> Result<String> {
    let normalized = normalize_summary(summary)?;
    append_entry(paths, "operational-note", "cli/manual", &normalized, now)?;
    Ok(normalized)
}

fn append_entry(
    paths: &PraxisPaths,
    kind: &str,
    source: &str,
    summary: &str,
    now: DateTime<Utc>,
) -> Result<()> {
    let mut file = fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(&paths.learnings_file)
        .with_context(|| format!("failed to open {}", paths.learnings_file.display()))?;
    writeln!(file)?;
    writeln!(file, "## {}", now.to_rfc3339())?;
    writeln!(file, "- Kind: {kind}")?;
    writeln!(file, "- Source: {source}")?;
    writeln!(file, "- Summary: {}", normalize_summary(summary)?)?;
    Ok(())
}

fn normalize_summary(summary: &str) -> Result<String> {
    let normalized = summary.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.is_empty() {
        bail!("learning summary cannot be empty");
    }
    Ok(normalized)
}
