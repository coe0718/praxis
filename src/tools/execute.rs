use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
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
        _ => Ok(fallback_result(
            manifest,
            request,
            "No execution adapter is installed for this tool yet.",
        )),
    }
}

fn append_text(
    paths: &PraxisPaths,
    manifest: &ToolManifest,
    request: &StoredApprovalRequest,
) -> Result<ToolExecutionResult> {
    let Some(raw_payload) = request.payload_json.as_deref() else {
        return Ok(fallback_result(
            manifest,
            request,
            "Legacy approved request had no structured payload, so Praxis kept this run safe and skipped the file mutation.",
        ));
    };
    let payload = parse_payload(Some(raw_payload))?;
    let text = payload
        .append_text
        .as_deref()
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .ok_or_else(|| anyhow::anyhow!("tool {} requires append_text payload", manifest.name))?;

    let plans = request
        .write_paths
        .iter()
        .map(|raw| plan_append(paths, raw, text))
        .collect::<Result<Vec<_>>>()?;

    for plan in plans {
        if let Some(block) = plan.block.as_deref() {
            append_block(&plan.path, block)?;
        }
    }

    Ok(ToolExecutionResult {
        summary: format!(
            "Executed approved tool {} and appended operator-approved text to {} file(s).",
            manifest.name,
            request.write_paths.len()
        ),
    })
}

#[derive(Debug)]
struct AppendPlan {
    path: PathBuf,
    block: Option<String>,
}

fn plan_append(paths: &PraxisPaths, raw: &str, text: &str) -> Result<AppendPlan> {
    let relative = normalize_relative(raw)?;
    ensure_safe_destination(&paths.data_dir, &relative)?;
    let full_path = paths.data_dir.join(&relative);
    if let Some(parent) = full_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    Ok(AppendPlan {
        block: render_append_block(&full_path, text)?,
        path: full_path,
    })
}

fn ensure_safe_destination(root: &Path, relative: &Path) -> Result<()> {
    let mut current = PathBuf::new();
    for component in relative.components() {
        current.push(component.as_os_str());
        let candidate = root.join(&current);
        if !candidate.exists() {
            continue;
        }
        let metadata = fs::symlink_metadata(&candidate)
            .with_context(|| format!("failed to inspect {}", candidate.display()))?;
        if metadata.file_type().is_symlink() {
            bail!(
                "refusing to write through symlink target {}",
                candidate.display()
            );
        }
    }
    Ok(())
}

fn render_append_block(path: &Path, text: &str) -> Result<Option<String>> {
    let existing = match fs::read(path) {
        Ok(existing) => existing,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(error) => {
            return Err(error).with_context(|| format!("failed to read {}", path.display()));
        }
    };
    let mut block = String::new();
    if !existing.is_empty() && !existing.ends_with(b"\n") {
        block.push('\n');
    }
    block.push_str(text);
    block.push('\n');
    if existing.ends_with(block.as_bytes()) {
        Ok(None)
    } else {
        Ok(Some(block))
    }
}

fn append_block(path: &Path, block: &str) -> Result<()> {
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .with_context(|| format!("failed to open {}", path.display()))?;
    write!(file, "{block}").with_context(|| format!("failed to append {}", path.display()))?;
    Ok(())
}

fn fallback_result(
    manifest: &ToolManifest,
    request: &StoredApprovalRequest,
    reason: &str,
) -> ToolExecutionResult {
    let rehearsal = if manifest.rehearsal_required {
        "Rehearsal required; "
    } else {
        ""
    };
    ToolExecutionResult {
        summary: format!(
            "{rehearsal}{reason} Stub execution recorded for approved tool {} with {} declared write paths. No external side effects were performed.",
            manifest.name,
            request.write_paths.len()
        ),
    }
}
