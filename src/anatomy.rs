use std::{fs, path::Path};

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

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
    let first_line = content
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("");
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
    if chars == 0 {
        0
    } else {
        chars.div_ceil(4) as i64
    }
}
