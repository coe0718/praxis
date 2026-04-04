use std::{
    fs::{self, OpenOptions},
    io::Write,
};

use anyhow::{Context, Result, bail};

use crate::{paths::PraxisPaths, storage::StoredApprovalRequest};

use super::{ToolManifest, policy::normalize_relative, request::parse_payload};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolExecutionResult {
    pub summary: String,
}

pub fn execute_request(
    paths: &PraxisPaths,
    manifest: &ToolManifest,
    request: &StoredApprovalRequest,
) -> Result<ToolExecutionResult> {
    match manifest.name.as_str() {
        "internal-maintenance" => Ok(ToolExecutionResult {
            summary: "Internal maintenance completed without external side effects.".to_string(),
        }),
        "praxis-data-write" => append_text(paths, manifest, request),
        _ => bail!(
            "tool {} is approved but has no execution adapter yet",
            manifest.name
        ),
    }
}

fn append_text(
    paths: &PraxisPaths,
    manifest: &ToolManifest,
    request: &StoredApprovalRequest,
) -> Result<ToolExecutionResult> {
    let payload = parse_payload(request.payload_json.as_deref())?;
    let text = payload
        .append_text
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("tool {} requires append_text payload", manifest.name))?;

    for relative in &request.write_paths {
        let relative = normalize_relative(relative)?;
        let full_path = paths.data_dir.join(&relative);
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        append_block(&full_path, text)?;
    }

    Ok(ToolExecutionResult {
        summary: format!(
            "Executed approved tool {} and appended operator-approved text to {} file(s).",
            manifest.name,
            request.write_paths.len()
        ),
    })
}

fn append_block(path: &std::path::Path, text: &str) -> Result<()> {
    let needs_separator = fs::read(path)
        .map(|existing| !existing.is_empty() && !existing.ends_with(b"\n"))
        .unwrap_or(false);
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .with_context(|| format!("failed to open {}", path.display()))?;

    if needs_separator {
        writeln!(file).with_context(|| format!("failed to write {}", path.display()))?;
    }
    writeln!(file, "{text}").with_context(|| format!("failed to append {}", path.display()))?;
    Ok(())
}
